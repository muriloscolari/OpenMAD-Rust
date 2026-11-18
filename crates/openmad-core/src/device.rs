//! USB HID transport and high-level keyboard operations for DuckBread boards.

use crate::devices::{describe, is_bootloader_pid, BoardSpec, UnlockedBoards, VENDOR_ID};
use crate::features::{
    decode_macro, encode_macro, join_macro_buffer, split_macro_buffer, AdvancedKey, BoxLight,
    DeadBand, Dks, KeyboardFeature, LightInfo, MacroStep, RapidTrigger, TapDance, DKS_ACTION_COUNT,
    DKS_ACTION_SIZE, LIGHT_EFFECTS,
};
use crate::profile::{fit_key_colors, key_str, Profile, COLOR_TOLERANCE, PER_KEY_EFFECT};
use crate::protocol::{
    build, build_buffer_read, build_buffer_write, build_vendor, decode_rgb, encode_rgb,
    keymap_size, travel_bytes, travel_from_bytes, AdvancedKeyMode, Cmd, FlashOp,
    KeyboardValue, Reply, SubField, Vendor, ANIMATED_EFFECTS, BRIGHTNESS_MAX, COLOUR_EFFECTS,
    CUSTOM_CHANNEL, DEFAULT_EFFECT_COLOUR, DEFAULT_EFFECT_SPEED, DEFAULT_LED_GAIN,
    KNOWN_PROTOCOL_VERSION, MAX_ACTUATION_READ, MAX_ACTUATION_WRITE, MAX_BUFFER_CHUNK, MAX_PAYLOAD,
    MAX_RT_READ, MAX_RT_WRITE, REPORT_SIZE, USAGE, USAGE_PAGE,
};
use hidapi::{HidApi, HidDevice};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;

const MAX_TELEMETRY_READ: usize = (REPORT_SIZE - 8) / 2; // 12
const MAX_RGB_PER_PACKET: usize = (REPORT_SIZE - 5) / 3; // 9

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("No DuckBread keyboard found (VID {VENDOR_ID:#06x}, usage {USAGE_PAGE:#06x}:{USAGE:#04x})")]
    NotFound,
    #[error("Keyboard is in bootloader mode; flash or replug via hub.fgg.com.cn")]
    InBootloader,
    #[error("Permission denied opening HID device. On Linux, install 60-openmad.rules in /etc/udev/rules.d/ and replug the keyboard")]
    PermissionDenied,
    #[error("Write refused: {0}")]
    WriteRefused(String),
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),
    #[error("{0}")]
    Protocol(String),
}

pub type Result<T> = std::result::Result<T, DeviceError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidInterfaceInfo {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
    pub product_string: String,
}

pub fn find_interfaces() -> Result<Vec<HidInterfaceInfo>> {
    let api = HidApi::new()?;
    let list = api
        .device_list()
        .filter(|d| d.vendor_id() == VENDOR_ID)
        .map(|d| HidInterfaceInfo {
            path: d.path().to_string_lossy().into_owned(),
            vendor_id: d.vendor_id(),
            product_id: d.product_id(),
            usage_page: d.usage_page(),
            usage: d.usage(),
            interface_number: d.interface_number(),
            product_string: d.product_string().unwrap_or("").to_string(),
        })
        .collect();
    Ok(list)
}

pub fn in_bootloader() -> bool {
    let Ok(api) = HidApi::new() else {
        return false;
    };
    let found = api
        .device_list()
        .any(|d| d.vendor_id() == VENDOR_ID && is_bootloader_pid(d.product_id()));
    found
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullConfig {
    pub protocol_version: Option<u16>,
    pub firmware_version: Option<u32>,
    pub board_name: String,
    pub product_id: u16,
    pub rows: usize,
    pub cols: usize,
    pub layers: usize,
    pub populated: Vec<bool>,
    pub actuation: Vec<f32>,
    pub rapid_trigger: Vec<RapidTrigger>,
    pub dead_band: DeadBand,
    pub feature: KeyboardFeature,
    pub game_mode: u8,
    pub bottom_optimize: u8,
    pub light: LightInfo,
    pub box_light: BoxLight,
    pub key_colors: Vec<[u8; 3]>,
    pub dks: Vec<Dks>,
    pub tap_dance: Vec<TapDance>,
    pub advanced_keys: Vec<AdvancedKey>,
    pub keymap: Vec<u8>,
    pub macros: Vec<Vec<MacroStep>>,
}

pub struct Mad68 {
    dev: HidDevice,
    pub spec: BoardSpec,
    pub writes: bool,
    pub dangerous: bool,
    pub timeout_ms: i32,
    pub gain: [f32; 3],
    pub protocol_version: Option<u16>,
    pub firmware_version: Option<u32>,
    pub read_only_reason: Option<String>,
    populated_cache: Option<Vec<bool>>,
}

impl Mad68 {
    pub fn open(
        writes: bool,
        dangerous: bool,
        gain: [f32; 3],
        unlocked: &UnlockedBoards,
    ) -> Result<Self> {
        let api = HidApi::new()?;
        let mut config_dev = None;
        let mut boot_found = false;

        for d in api.device_list() {
            if d.vendor_id() != VENDOR_ID {
                continue;
            }
            if is_bootloader_pid(d.product_id()) {
                boot_found = true;
                continue;
            }
            if d.usage_page() == USAGE_PAGE && d.usage() == USAGE {
                config_dev = Some((d.path().to_owned(), d.product_id()));
                break;
            }
        }

        // On Linux hidraw, usage_page/usage may not be populated by hidapi;
        // inspect the sysfs report_descriptor for the VIA header [0x06, 0x60, 0xFF, 0x09, 0x61].
        #[cfg(target_os = "linux")]
        if config_dev.is_none() {
            const VIA_HEADER: [u8; 5] = [0x06, 0x60, 0xFF, 0x09, 0x61];
            for d in api.device_list() {
                if d.vendor_id() != VENDOR_ID || is_bootloader_pid(d.product_id()) {
                    continue;
                }
                let path_str = d.path().to_string_lossy();
                if let Some(dev_name) = std::path::Path::new(path_str.as_ref())
                    .file_name()
                    .and_then(|s| s.to_str())
                {
                    let desc_path = format!("/sys/class/hidraw/{dev_name}/device/report_descriptor");
                    if let Ok(desc) = std::fs::read(desc_path) {
                        if desc.starts_with(&VIA_HEADER) {
                            config_dev = Some((d.path().to_owned(), d.product_id()));
                            break;
                        }
                    }
                }
            }
        }

        // Fallback on Linux hidraw where neither usage_page nor sysfs matched:
        // pick interface_number == 1 if present.
        if config_dev.is_none() {
            for d in api.device_list() {
                if d.vendor_id() == VENDOR_ID
                    && !is_bootloader_pid(d.product_id())
                    && d.interface_number() == 1
                {
                    config_dev = Some((d.path().to_owned(), d.product_id()));
                    break;
                }
            }
        }

        let Some((path, pid)) = config_dev else {
            return Err(if boot_found {
                DeviceError::InBootloader
            } else {
                DeviceError::NotFound
            });
        };

        let dev = api.open_path(&path).map_err(|e| {
            let msg = e.to_string().to_lowercase();
            if msg.contains("permission") || msg.contains("access") || msg.contains("eacces") {
                return DeviceError::PermissionDenied;
            }
            // libhidapi-hidraw often returns "hid_error is not implemented yet" on EACCES;
            // probe the underlying path directly with std::fs to detect PermissionDenied.
            let path_str = path.to_string_lossy();
            if let Err(io_err) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path_str.as_ref())
            {
                if io_err.kind() == std::io::ErrorKind::PermissionDenied {
                    return DeviceError::PermissionDenied;
                }
            }
            DeviceError::Hid(e)
        })?;

        let spec = describe(pid);
        let mut kb = Self {
            dev,
            spec,
            writes,
            dangerous,
            timeout_ms: 1000,
            gain,
            protocol_version: None,
            firmware_version: None,
            read_only_reason: None,
            populated_cache: None,
        };

        if let Ok(pv) = kb.read_protocol_version() {
            kb.protocol_version = Some(pv);
        }
        if let Ok(r) = kb.keyboard_value(KeyboardValue::FirmwareVersion) {
            let p = r.payload();
            if p.len() >= 5 {
                kb.firmware_version = Some(u32::from_be_bytes([p[1], p[2], p[3], p[4]]));
            }
        }

        if !kb.spec.verified && !unlocked.is_unlocked(kb.spec.product_id) {
            kb.read_only_reason = Some(format!(
                "{} ({:#06x}) is unverified; opened read-only",
                kb.spec.name, kb.spec.product_id
            ));
            kb.writes = false;
        }

        Ok(kb)
    }

    pub fn set_gain(&mut self, gain: [f32; 3]) {
        self.gain = gain;
    }

    pub fn firmware_verified(&self) -> bool {
        self.protocol_version == Some(KNOWN_PROTOCOL_VERSION)
    }

    pub fn total_keys(&self) -> usize {
        self.spec.total_keys()
    }

    pub fn keymap_size(&self) -> usize {
        keymap_size(self.spec.rows, self.spec.cols, self.spec.layers)
    }

    pub fn key_index(&self, row: usize, col: usize) -> usize {
        row * self.spec.cols + col
    }

    pub fn key_rowcol(&self, index: usize) -> (usize, usize) {
        (index / self.spec.cols, index % self.spec.cols)
    }

    fn check_packet(&self, packet: &[u8; REPORT_SIZE]) -> Result<()> {
        let Some(cmd) = Cmd::from_u8(packet[0]) else {
            return Ok(());
        };
        if cmd.is_dangerous() && !self.dangerous {
            return Err(DeviceError::WriteRefused(format!(
                "dangerous command {cmd:?} requires explicit opt-in"
            )));
        }
        if packet[1] == CUSTOM_CHANNEL {
            if let Some(sub) = Vendor::from_u8(packet[2]) {
                if sub.is_dangerous() && !self.dangerous {
                    return Err(DeviceError::WriteRefused(format!(
                        "dangerous vendor sub-command {sub:?} requires explicit opt-in"
                    )));
                }
            }
        }
        if cmd.is_write() && !self.writes {
            if let Some(reason) = &self.read_only_reason {
                return Err(DeviceError::WriteRefused(reason.clone()));
            }
            return Err(DeviceError::WriteRefused(
                "device handle is read-only".to_string(),
            ));
        }
        Ok(())
    }

    pub fn send(&self, packet: &[u8; REPORT_SIZE], expect_reply: bool) -> Result<Option<Reply>> {
        self.check_packet(packet)?;

        let mut out = [0u8; REPORT_SIZE + 1];
        out[0] = 0x00; // report ID 0
        out[1..].copy_from_slice(packet);
        self.dev.write(&out)?;

        if !expect_reply {
            return Ok(None);
        }

        let want = packet[0];
        let want_vendor: Option<&[u8]> = if packet[1] == CUSTOM_CHANNEL {
            Some(&packet[1..3])
        } else if want == Cmd::CustomGetValue as u8 || want == Cmd::CustomSetValue as u8 {
            Some(&packet[1..2])
        } else {
            None
        };

        let deadline = Instant::now() + Duration::from_millis(self.timeout_ms as u64);
        let mut buf = [0u8; REPORT_SIZE];
        loop {
            let rem = deadline.saturating_duration_since(Instant::now());
            if rem.is_zero() {
                break;
            }
            let n = self.dev.read_timeout(&mut buf, rem.as_millis() as i32)?;
            if n == 0 {
                continue;
            }
            let reply = Reply::from_slice(&buf[..n]);
            let matches = reply.command_id() == want
                && match want_vendor {
                    None => true,
                    Some(v) => &reply.raw[1..1 + v.len()] == v,
                };
            if matches {
                return Ok(Some(reply));
            }
            if reply.is_unhandled() {
                return Err(DeviceError::Protocol(format!(
                    "firmware reported command {want:#04x} as unhandled"
                )));
            }
        }

        Err(DeviceError::Protocol(format!(
            "timed out waiting for reply to command {want:#04x}"
        )))
    }

    pub fn cmd(&self, cmd: Cmd, payload: &[u8]) -> Result<Reply> {
        let pkt = build(cmd, payload).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?
            .ok_or_else(|| DeviceError::Protocol("missing reply".into()))
    }

    pub fn vendor_get(&self, sub: Vendor, payload: &[u8]) -> Result<Reply> {
        let pkt = build_vendor(Cmd::GetKeyboardValue, sub, payload).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?
            .ok_or_else(|| DeviceError::Protocol("missing reply".into()))
    }

    pub fn vendor_set(&self, sub: Vendor, payload: &[u8]) -> Result<Reply> {
        let pkt = build_vendor(Cmd::SetKeyboardValue, sub, payload).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?
            .ok_or_else(|| DeviceError::Protocol("missing reply".into()))
    }

    pub fn read_protocol_version(&self) -> Result<u16> {
        let r = self.cmd(Cmd::GetProtocolVersion, &[])?;
        let p = r.payload();
        Ok(u16::from_be_bytes([p[0], p[1]]))
    }

    pub fn layer_count(&self) -> Result<usize> {
        let r = self.cmd(Cmd::DynamicKeymapGetLayerCount, &[])?;
        Ok(r.payload()[0] as usize)
    }

    pub fn macro_count(&self) -> Result<usize> {
        let r = self.cmd(Cmd::DynamicKeymapMacroGetCount, &[])?;
        Ok(r.payload()[0] as usize)
    }

    pub fn macro_buffer_size(&self) -> Result<usize> {
        let r = self.cmd(Cmd::DynamicKeymapMacroGetBufferSize, &[])?;
        let p = r.payload();
        Ok(u16::from_be_bytes([p[0], p[1]]) as usize)
    }

    pub fn keyboard_value(&self, val: KeyboardValue) -> Result<Reply> {
        self.cmd(Cmd::GetKeyboardValue, &[val as u8])
    }

    pub fn read_buffer(&self, cmd: Cmd, size: usize, base: usize) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(size);
        let mut off = 0;
        while off < size {
            let chunk = (size - off).min(MAX_BUFFER_CHUNK) as u8;
            let pkt = build_buffer_read(cmd, (base + off) as u16, chunk)
                .map_err(DeviceError::Protocol)?;
            let r = self
                .send(&pkt, true)?
                .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
            let data = r.buffer_data();
            let take = data.len().min(chunk as usize);
            out.extend_from_slice(&data[..take]);
            off += take;
            if take == 0 {
                break;
            }
        }
        Ok(out)
    }

    pub fn write_buffer(&self, cmd: Cmd, data: &[u8], base: usize) -> Result<()> {
        let mut off = 0;
        while off < data.len() {
            let end = (off + MAX_BUFFER_CHUNK).min(data.len());
            let pkt = build_buffer_write(cmd, (base + off) as u16, &data[off..end])
                .map_err(DeviceError::Protocol)?;
            self.send(&pkt, true)?;
            off = end;
        }
        Ok(())
    }

    pub fn read_keymap(&self) -> Result<Vec<u8>> {
        self.read_buffer(Cmd::DynamicKeymapGetBuffer, self.keymap_size(), 0)
    }

    pub fn write_keymap(&self, data: &[u8]) -> Result<()> {
        let expected = self.keymap_size();
        if data.len() != expected {
            return Err(DeviceError::Protocol(format!(
                "keymap length {} != expected {expected}",
                data.len()
            )));
        }
        self.write_buffer(Cmd::DynamicKeymapSetBuffer, data, 0)
    }

    pub fn write_keycode(&self, layer: usize, row: usize, col: usize, keycode: u16) -> Result<()> {
        let kc = keycode.to_be_bytes();
        self.cmd(
            Cmd::DynamicKeymapSetKeycode,
            &[layer as u8, row as u8, col as u8, kc[0], kc[1]],
        )?;
        Ok(())
    }

    fn bulk_request(&self, sub: Vendor, offset: usize, count: usize) -> Result<[u8; REPORT_SIZE]> {
        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = CUSTOM_CHANNEL;
        payload[1] = sub as u8;
        let off = (offset as u16).to_be_bytes();
        payload[4] = off[0];
        payload[5] = off[1];
        payload[6] = count as u8;
        build(Cmd::GetKeyboardValue, &payload).map_err(DeviceError::Protocol)
    }

    fn bulk_write(
        &self,
        sub: Vendor,
        offset: usize,
        count: usize,
        data: &[u8],
        flash_op: FlashOp,
    ) -> Result<()> {
        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = CUSTOM_CHANNEL;
        payload[1] = sub as u8;
        let off = (offset as u16).to_be_bytes();
        payload[4] = off[0];
        payload[5] = off[1];
        payload[6] = count as u8;
        payload[7] = flash_op as u8;
        payload[8..8 + data.len()].copy_from_slice(data);
        let pkt = build(Cmd::SetKeyboardValue, &payload).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?;
        Ok(())
    }

    pub fn read_actuation_bulk(&self) -> Result<Vec<f32>> {
        let total = self.total_keys();
        let mut out = Vec::with_capacity(total);
        while out.len() < total {
            let want = MAX_ACTUATION_READ.min(total - out.len());
            let pkt = self.bulk_request(Vendor::BufferTogTh, out.len(), want)?;
            let r = self
                .send(&pkt, true)?
                .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
            for i in 0..want {
                let base = 8 + i * 2;
                out.push(travel_from_bytes(r.raw[base], r.raw[base + 1]));
            }
        }
        Ok(out)
    }

    pub fn write_actuation_bulk(
        &self,
        values: &[f32],
        offset: usize,
        flash_op: FlashOp,
    ) -> Result<()> {
        let mut i = 0;
        while i < values.len() {
            let end = (i + MAX_ACTUATION_WRITE).min(values.len());
            let chunk = &values[i..end];
            let mut data = Vec::with_capacity(chunk.len() * 2);
            for &mm in chunk {
                data.extend_from_slice(&travel_bytes(mm));
            }
            self.bulk_write(Vendor::BufferTogTh, offset + i, chunk.len(), &data, flash_op)?;
            i = end;
        }
        Ok(())
    }

    pub fn read_rapid_trigger_bulk(&self) -> Result<Vec<RapidTrigger>> {
        let total = self.total_keys();
        let mut out = Vec::with_capacity(total);
        while out.len() < total {
            let want = MAX_RT_READ.min(total - out.len());
            let base_idx = out.len();
            let pkt = self.bulk_request(Vendor::BufferRt, base_idx, want)?;
            let r = self
                .send(&pkt, true)?
                .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
            for i in 0..want {
                let b = 8 + i * 5;
                let (row, col) = self.key_rowcol(base_idx + i);
                out.push(RapidTrigger {
                    row: row as u8,
                    col: col as u8,
                    enabled: r.raw[b] != 0,
                    release_mm: travel_from_bytes(r.raw[b + 1], r.raw[b + 2]),
                    press_mm: travel_from_bytes(r.raw[b + 3], r.raw[b + 4]),
                });
            }
        }
        Ok(out)
    }

    pub fn write_rapid_trigger_bulk(
        &self,
        specs: &[RapidTrigger],
        offset: usize,
        flash_op: FlashOp,
    ) -> Result<()> {
        let mut i = 0;
        while i < specs.len() {
            let end = (i + MAX_RT_WRITE).min(specs.len());
            let chunk = &specs[i..end];
            let mut data = Vec::with_capacity(chunk.len() * 5);
            for rt in chunk {
                data.push(if rt.enabled { 1 } else { 0 });
                data.extend_from_slice(&travel_bytes(rt.release_mm));
                data.extend_from_slice(&travel_bytes(rt.press_mm));
            }
            self.bulk_write(Vendor::BufferRt, offset + i, chunk.len(), &data, flash_op)?;
            i = end;
        }
        Ok(())
    }

    fn read_u16_bulk(&self, sub: Vendor, count: usize) -> Result<Vec<u16>> {
        let mut out = Vec::with_capacity(count);
        while out.len() < count {
            let want = MAX_TELEMETRY_READ.min(count - out.len());
            let mut last_err = None;
            let mut reply = None;
            for _ in 0..3 {
                let pkt = self.bulk_request(sub, out.len(), want)?;
                match self.send(&pkt, true) {
                    Ok(Some(r)) => {
                        reply = Some(r);
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => last_err = Some(e),
                }
            }
            let r = match reply {
                Some(r) => r,
                None => return Err(last_err.unwrap_or_else(|| DeviceError::Protocol("telemetry failed".into()))),
            };
            for i in 0..want {
                let b = 8 + i * 2;
                out.push(u16::from_be_bytes([r.raw[b], r.raw[b + 1]]));
            }
        }
        Ok(out)
    }

    pub fn read_adc_raw(&self) -> Result<Vec<u16>> {
        self.read_u16_bulk(Vendor::RealtimeAdcAxleBuffer, self.total_keys())
    }

    pub fn read_trip_raw(&self) -> Result<Vec<u16>> {
        self.read_u16_bulk(Vendor::RealtimeTripAxleBuffer, self.total_keys())
    }

    pub fn read_trip_mm(&self, row: usize, col: usize) -> Result<f32> {
        let r = self.vendor_get(Vendor::RealtimeTripAxle, &[row as u8, col as u8])?;
        let p = r.payload();
        Ok(travel_from_bytes(p[4], p[5]))
    }

    pub fn read_calibration_status(&self) -> Result<Vec<u8>> {
        let total = self.total_keys();
        let mut out = Vec::with_capacity(total);
        let per_packet = REPORT_SIZE - 8;
        while out.len() < total {
            let want = per_packet.min(total - out.len());
            let pkt = self.bulk_request(Vendor::Calibration, out.len(), want)?;
            let r = self
                .send(&pkt, true)?
                .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
            out.extend_from_slice(&r.raw[8..8 + want]);
        }
        Ok(out)
    }

    pub fn populated_mask(&mut self) -> Vec<bool> {
        if let Some(ref mask) = self.populated_cache {
            return mask.clone();
        }
        let mask = match self.read_calibration_status() {
            Ok(v) => v.into_iter().map(|x| x != 0).collect(),
            Err(_) => vec![true; self.total_keys()],
        };
        self.populated_cache = Some(mask.clone());
        mask
    }

    pub fn start_calibration(&self) -> Result<()> {
        self.vendor_set(Vendor::CalibrationStart, &[])?;
        Ok(())
    }

    pub fn finish_calibration(&self) -> Result<()> {
        self.vendor_set(Vendor::CalibrationFinish, &[])?;
        Ok(())
    }

    pub fn factory_reset(&self) -> Result<()> {
        self.vendor_set(Vendor::ResetAll, &[])?;
        Ok(())
    }

    pub fn read_dks(&self, index: usize) -> Result<Dks> {
        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = CUSTOM_CHANNEL;
        payload[1] = Vendor::Dks as u8;
        payload[4] = SubField::Get as u8;
        payload[5] = index as u8;
        let pkt = build(Cmd::GetKeyboardValue, &payload).map_err(DeviceError::Protocol)?;
        let r = self
            .send(&pkt, true)?
            .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
        let end = 7 + DKS_ACTION_COUNT * DKS_ACTION_SIZE;
        Ok(Dks::from_bytes(index, &r.raw[7..end]))
    }

    pub fn write_dks(&self, dks: &Dks) -> Result<()> {
        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = CUSTOM_CHANNEL;
        payload[1] = Vendor::Dks as u8;
        payload[4] = SubField::Set as u8;
        payload[5] = dks.index as u8;
        let body = dks.to_bytes();
        payload[6..6 + body.len()].copy_from_slice(&body);
        let pkt = build(Cmd::SetKeyboardValue, &payload).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?;
        Ok(())
    }

    pub fn read_tap_dance(&self, index: usize) -> Result<TapDance> {
        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = Vendor::BufferTogTh as u8; // ENTRY_OP 0x0D
        payload[1] = Vendor::TapDanceGet as u8;
        payload[2] = index as u8;
        let pkt = build(Cmd::VialPrefix, &payload).map_err(DeviceError::Protocol)?;
        let r = self
            .send(&pkt, true)?
            .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
        Ok(TapDance {
            index: r.raw[3] as usize,
            tap: u16::from_be_bytes([r.raw[4], r.raw[5]]),
            hold: u16::from_be_bytes([r.raw[6], r.raw[7]]),
            timer_ms: u16::from_be_bytes([r.raw[8], r.raw[9]]),
        })
    }

    pub fn write_tap_dance(&self, td: &TapDance) -> Result<()> {
        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = Vendor::BufferTogTh as u8;
        payload[1] = Vendor::TapDanceSet as u8;
        payload[2] = td.index as u8;
        let tap = td.tap.to_be_bytes();
        let hold = td.hold.to_be_bytes();
        let timer = td.timer_ms.to_be_bytes();
        payload[3] = tap[0];
        payload[4] = tap[1];
        payload[5] = hold[0];
        payload[6] = hold[1];
        payload[7] = timer[0];
        payload[8] = timer[1];
        let pkt = build(Cmd::VialPrefix, &payload).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?;
        Ok(())
    }

    pub fn read_advanced_key(&self, index: usize) -> Result<AdvancedKey> {
        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = CUSTOM_CHANNEL;
        payload[1] = Vendor::Rs as u8;
        payload[4] = SubField::Get as u8;
        payload[5] = index as u8;
        payload[6] = index as u8;
        let pkt = build(Cmd::GetKeyboardValue, &payload).map_err(DeviceError::Protocol)?;
        let r = self
            .send(&pkt, true)?
            .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
        Ok(AdvancedKey {
            index: r.raw[6] as usize,
            mode: AdvancedKeyMode::from_u8(r.raw[7]),
            id: r.raw[8],
            rs_apc_lv: u16::from_be_bytes([r.raw[9], r.raw[10]]),
            gapc_sw: r.raw[11],
            rt_sw: r.raw[12],
            key1_row: r.raw[13],
            key1_col: r.raw[14],
            key2_row: r.raw[15],
            key2_col: r.raw[16],
            layer: r.raw[17],
        })
    }

    pub fn write_advanced_key(&self, key: &AdvancedKey) -> Result<()> {
        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = CUSTOM_CHANNEL;
        payload[1] = Vendor::Rs as u8;
        payload[4] = SubField::Set as u8;
        payload[5] = key.index as u8;
        payload[6] = key.mode as u8;
        payload[7] = if key.id >= 0x1A { key.id - 0x1A } else { key.id };
        let apc = key.rs_apc_lv.to_be_bytes();
        payload[8] = apc[0];
        payload[9] = apc[1];
        payload[10] = key.gapc_sw;
        payload[11] = key.rt_sw;
        payload[12] = key.key1_row;
        payload[13] = key.key1_col;
        payload[14] = key.key2_row;
        payload[15] = key.key2_col;
        payload[16] = key.layer;
        let pkt = build(Cmd::SetKeyboardValue, &payload).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?;
        Ok(())
    }

    pub fn read_key_colors(&self, row: usize, col: usize, count: usize) -> Result<Vec<[u8; 3]>> {
        let n = count.min(MAX_RGB_PER_PACKET);
        let pkt = build(
            Cmd::CustomGetValue,
            &[Vendor::GetCustomLamplight as u8, row as u8, col as u8, n as u8],
        )
        .map_err(DeviceError::Protocol)?;
        let r = self
            .send(&pkt, true)?
            .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let b = 5 + i * 3;
            out.push(decode_rgb([r.raw[b], r.raw[b + 1], r.raw[b + 2]], self.gain));
        }
        Ok(out)
    }

    pub fn write_key_colors(&self, row: usize, col: usize, colors: &[[u8; 3]]) -> Result<()> {
        if colors.len() > MAX_RGB_PER_PACKET {
            return Err(DeviceError::Protocol("too many colors in packet".into()));
        }
        let mut body = Vec::with_capacity(4 + colors.len() * 3);
        body.push(Vendor::SetCustomLamplight as u8);
        body.push(row as u8);
        body.push(col as u8);
        body.push(colors.len() as u8);
        for c in colors {
            body.extend_from_slice(&encode_rgb(c[0], c[1], c[2], self.gain));
        }
        let pkt = build(Cmd::CustomSetValue, &body).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?;
        Ok(())
    }

    pub fn read_all_key_colors(&self) -> Result<Vec<[u8; 3]>> {
        let total = self.total_keys();
        let mut out = Vec::with_capacity(total);
        while out.len() < total {
            let idx = out.len();
            let want = MAX_RGB_PER_PACKET.min(total - idx);
            let (r, c) = self.key_rowcol(idx);
            out.extend(self.read_key_colors(r, c, want)?);
        }
        Ok(out)
    }

    pub fn write_all_key_colors(&self, colors: &[[u8; 3]]) -> Result<()> {
        let limit = colors.len().min(self.total_keys());
        let mut i = 0;
        while i < limit {
            let end = (i + MAX_RGB_PER_PACKET).min(limit);
            let (r, c) = self.key_rowcol(i);
            self.write_key_colors(r, c, &colors[i..end])?;
            i = end;
        }
        Ok(())
    }

    pub fn save_lighting(&self) -> Result<()> {
        self.vendor_set(Vendor::SaveLamplight, &[])?;
        Ok(())
    }

    pub fn read_light_info(&self) -> Result<LightInfo> {
        let pkt = build(Cmd::CustomGetValue, &[Vendor::LightInfo as u8]).map_err(DeviceError::Protocol)?;
        let r = self
            .send(&pkt, true)?
            .ok_or_else(|| DeviceError::Protocol("missing reply".into()))?;
        let rgb = decode_rgb([r.raw[5], r.raw[6], r.raw[7]], self.gain);
        Ok(LightInfo {
            effect: r.raw[2],
            speed: r.raw[4],
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
            brightness: r.raw[8],
        })
    }

    pub fn write_light_info(&self, info: &LightInfo) -> Result<()> {
        let meta = LIGHT_EFFECTS.iter().find(|e| e.id == info.effect);
        let has_speed = meta.map_or(ANIMATED_EFFECTS.contains(&info.effect), |m| m.has_speed);
        let has_color = meta.map_or(COLOUR_EFFECTS.contains(&info.effect), |m| m.has_color);

        let speed = if has_speed && info.speed == 0 {
            DEFAULT_EFFECT_SPEED
        } else {
            info.speed
        };
        let (r, g, b) = if has_color && info.r == 0 && info.g == 0 && info.b == 0 {
            DEFAULT_EFFECT_COLOUR
        } else {
            (info.r, info.g, info.b)
        };

        let mut payload = [0u8; MAX_PAYLOAD];
        payload[0] = Vendor::LightInfo as u8;
        payload[1] = info.effect;
        payload[3] = speed;
        let rgb = encode_rgb(r, g, b, self.gain);
        payload[4] = rgb[0];
        payload[5] = rgb[1];
        payload[6] = rgb[2];
        payload[7] = info.brightness.min(BRIGHTNESS_MAX);
        let pkt = build(Cmd::CustomSetValue, &payload).map_err(DeviceError::Protocol)?;
        self.send(&pkt, true)?;
        Ok(())
    }

    pub fn read_dead_band(&self) -> Result<DeadBand> {
        let r = self.vendor_get(Vendor::DeadBand, &[])?;
        let d = &r.payload()[2..];
        Ok(DeadBand {
            top_mm: travel_from_bytes(d[0], d[1]),
            bottom_mm: travel_from_bytes(d[2], d[3]),
        })
    }

    pub fn write_dead_band(&self, band: &DeadBand) -> Result<()> {
        let top = travel_bytes(band.top_mm);
        let bot = travel_bytes(band.bottom_mm);
        self.vendor_set(Vendor::DeadBand, &[top[0], top[1], bot[0], bot[1]])?;
        Ok(())
    }

    pub fn read_feature(&self) -> Result<KeyboardFeature> {
        let r = self.vendor_get(Vendor::Feature, &[])?;
        let d = &r.payload()[2..];
        Ok(KeyboardFeature {
            rgb_area: d[0],
            wasd_switch: d[1],
            mac_switch: d[2],
            win_lock: d[3],
            nkro_switch: d[4],
        })
    }

    pub fn write_feature(&self, f: &KeyboardFeature) -> Result<()> {
        self.vendor_set(
            Vendor::Feature,
            &[f.rgb_area, f.wasd_switch, f.mac_switch, f.win_lock, f.nkro_switch],
        )?;
        Ok(())
    }

    pub fn read_game_mode(&self) -> Result<u8> {
        let r = self.vendor_get(Vendor::GameMode, &[])?;
        Ok(r.payload()[2])
    }

    pub fn write_game_mode(&self, mode: u8) -> Result<()> {
        self.vendor_set(Vendor::GameMode, &[mode])?;
        Ok(())
    }

    pub fn read_bottom_optimize(&self) -> Result<u8> {
        let r = self.vendor_get(Vendor::BottomOptimizeSwitch, &[])?;
        Ok(r.payload()[2])
    }

    pub fn write_bottom_optimize(&self, opt: u8) -> Result<()> {
        self.vendor_set(Vendor::BottomOptimizeSwitch, &[opt])?;
        Ok(())
    }

    pub fn read_box_light(&self) -> Result<BoxLight> {
        let r = self.vendor_get(Vendor::BoxLight, &[])?;
        let d = &r.payload()[2..];
        Ok(BoxLight {
            mode: d[0],
            colorful: d[1],
            brightness: d[2],
            speed: d[3],
        })
    }

    pub fn write_box_light(&self, b: &BoxLight) -> Result<()> {
        self.vendor_set(
            Vendor::BoxLight,
            &[b.mode, b.colorful, b.brightness, b.speed],
        )?;
        Ok(())
    }

    pub fn read_macro_steps(&self) -> Result<Vec<Vec<MacroStep>>> {
        let count = self.macro_count()?;
        let size = self.macro_buffer_size()?;
        let buf = self.read_buffer(Cmd::DynamicKeymapMacroGetBuffer, size, 0)?;
        Ok(split_macro_buffer(&buf, count)
            .into_iter()
            .map(|m| decode_macro(&m))
            .collect())
    }

    pub fn write_macro_slot(&self, index: usize, steps: &[MacroStep]) -> Result<()> {
        let count = self.macro_count()?;
        if index >= count {
            return Err(DeviceError::Protocol(format!("macro index {index} >= {count}")));
        }
        let size = self.macro_buffer_size()?;
        let current = self.read_buffer(Cmd::DynamicKeymapMacroGetBuffer, size, 0)?;
        let mut all: Vec<Vec<u8>> = split_macro_buffer(&current, count);
        all[index] = encode_macro(steps).map_err(DeviceError::Protocol)?;

        // Preserve any non-empty tail after the managed NUL-terminated region.
        let mut pos = 0;
        let mut seen = 0;
        while seen < count && pos < current.len() {
            if let Some(rel) = current[pos..].iter().position(|&b| b == 0) {
                pos += rel + 1;
                seen += 1;
            } else {
                pos = current.len();
                break;
            }
        }
        let tail = &current[pos..];
        let mut buf = join_macro_buffer(&all, size).map_err(DeviceError::Protocol)?;
        let used: usize = all.iter().map(|m| m.len() + 1).sum();
        let avail = size.saturating_sub(used);
        let copy_len = avail.min(tail.len());
        buf[used..used + copy_len].copy_from_slice(&tail[..copy_len]);

        self.write_buffer(Cmd::DynamicKeymapMacroSetBuffer, &buf, 0)
    }

    pub fn commit_to_flash(&self) -> Result<()> {
        let act = self.read_actuation_bulk()?;
        let rt = self.read_rapid_trigger_bulk()?;
        self.write_actuation_bulk(&act, 0, FlashOp::EraseAndWrite)?;
        self.write_rapid_trigger_bulk(&rt, 0, FlashOp::EraseAndWrite)?;
        self.save_lighting()?;
        Ok(())
    }

    pub fn read_full_config(&mut self) -> Result<FullConfig> {
        let populated = self.populated_mask();
        let actuation = self.read_actuation_bulk()?;
        let rapid_trigger = self.read_rapid_trigger_bulk()?;
        let dead_band = self.read_dead_band().unwrap_or_default();
        let feature = self.read_feature().unwrap_or_default();
        let game_mode = self.read_game_mode().unwrap_or(0);
        let bottom_optimize = self.read_bottom_optimize().unwrap_or(0);
        let light = self.read_light_info().unwrap_or_default();
        let box_light = self.read_box_light().unwrap_or_default();
        let key_colors = self.read_all_key_colors().unwrap_or_default();

        let mut dks = Vec::with_capacity(16);
        for i in 0..16 {
            if let Ok(d) = self.read_dks(i) {
                dks.push(d);
            }
        }
        let mut tap_dance = Vec::with_capacity(16);
        for i in 0..16 {
            if let Ok(td) = self.read_tap_dance(i) {
                tap_dance.push(td);
            }
        }
        let mut advanced_keys = Vec::with_capacity(16);
        for i in 0..16 {
            if let Ok(ak) = self.read_advanced_key(i) {
                advanced_keys.push(ak);
            }
        }
        let keymap = self.read_keymap().unwrap_or_default();
        let macros = self.read_macro_steps().unwrap_or_default();

        Ok(FullConfig {
            protocol_version: self.protocol_version,
            firmware_version: self.firmware_version,
            board_name: self.spec.name.clone(),
            product_id: self.spec.product_id,
            rows: self.spec.rows,
            cols: self.spec.cols,
            layers: self.spec.layers,
            populated,
            actuation,
            rapid_trigger,
            dead_band,
            feature,
            game_mode,
            bottom_optimize,
            light,
            box_light,
            key_colors,
            dks,
            tap_dance,
            advanced_keys,
            keymap,
            macros,
        })
    }

    /// Apply a profile using minimal-diff writes and `FlashOp::Normal` (or
    /// `EraseAndWrite` when explicitly saving onboard).
    pub fn apply_profile(&mut self, profile: &Profile, flash_op: FlashOp) -> Result<usize> {
        let mut writes = 0usize;
        let rows = self.spec.rows;
        let cols = self.spec.cols;
        let total = self.total_keys();

        // 1. Actuation
        let current_act = self.read_actuation_bulk()?;
        let mut desired_act = Vec::with_capacity(total);
        let mut act_changed = false;
        for idx in 0..total {
            let (r, c) = self.key_rowcol(idx);
            let want = profile.actuation_for(r, c);
            if (current_act[idx] - want).abs() > 0.005 {
                act_changed = true;
            }
            desired_act.push(want);
        }
        if act_changed || flash_op == FlashOp::EraseAndWrite {
            self.write_actuation_bulk(&desired_act, 0, flash_op)?;
            writes += 1;
        }

        // 2. Rapid Trigger
        let current_rt = self.read_rapid_trigger_bulk()?;
        let mut desired_rt = Vec::with_capacity(total);
        let mut rt_changed = false;
        for idx in 0..total {
            let (r, c) = self.key_rowcol(idx);
            let spec = profile.rapid_trigger_for(r, c);
            let cur = &current_rt[idx];
            if cur.enabled != spec.enabled
                || (cur.press_mm - spec.press_mm).abs() > 0.005
                || (cur.release_mm - spec.release_mm).abs() > 0.005
            {
                rt_changed = true;
            }
            desired_rt.push(RapidTrigger {
                row: r as u8,
                col: c as u8,
                enabled: spec.enabled,
                press_mm: spec.press_mm,
                release_mm: spec.release_mm,
            });
        }
        if rt_changed || flash_op == FlashOp::EraseAndWrite {
            self.write_rapid_trigger_bulk(&desired_rt, 0, flash_op)?;
            writes += 1;
        }

        // 3. Scalar settings
        if let Some(ref db) = profile.dead_band {
            self.write_dead_band(db)?;
            writes += 1;
        }
        if let Some(ref feat) = profile.feature {
            self.write_feature(feat)?;
            writes += 1;
        }
        if let Some(gm) = profile.game_mode {
            self.write_game_mode(gm)?;
            writes += 1;
        }
        if let Some(bo) = profile.bottom_optimize {
            self.write_bottom_optimize(bo)?;
            writes += 1;
        }

        // 4. Lighting
        if let Some(mut light) = profile.light {
            if ANIMATED_EFFECTS.contains(&light.effect) && light.speed == 0 {
                light.speed = DEFAULT_EFFECT_SPEED;
            }
            if COLOUR_EFFECTS.contains(&light.effect) && light.r == 0 && light.g == 0 && light.b == 0
            {
                light.r = DEFAULT_EFFECT_COLOUR.0;
                light.g = DEFAULT_EFFECT_COLOUR.1;
                light.b = DEFAULT_EFFECT_COLOUR.2;
            }
            self.write_light_info(&light)?;
            writes += 1;

            if light.effect == PER_KEY_EFFECT {
                if let Some(ref raw_colors) = profile.key_colors {
                    let fitted = fit_key_colors(raw_colors, rows, cols);
                    let cur_colors = self.read_all_key_colors().unwrap_or_default();
                    let mask = self.populated_mask();
                    let diff = fitted.iter().enumerate().any(|(i, c)| {
                        mask.get(i).copied().unwrap_or(true)
                            && cur_colors.get(i).map_or(true, |cur| {
                                (cur[0] as i16 - c[0] as i16).abs() > COLOR_TOLERANCE
                                    || (cur[1] as i16 - c[1] as i16).abs() > COLOR_TOLERANCE
                                    || (cur[2] as i16 - c[2] as i16).abs() > COLOR_TOLERANCE
                            })
                    });
                    if diff {
                        self.write_all_key_colors(&fitted)?;
                        writes += 1;
                    }
                }
            }
        }
        if let Some(ref box_l) = profile.box_light {
            self.write_box_light(box_l)?;
            writes += 1;
        }

        // 5. Advanced keys / DKS / TapDance
        if let Some(ref dks_list) = profile.dks {
            for d in dks_list {
                self.write_dks(d)?;
                writes += 1;
            }
        }
        if let Some(ref td_list) = profile.tap_dance {
            for td in td_list {
                self.write_tap_dance(td)?;
                writes += 1;
            }
        }
        if let Some(ref ak_list) = profile.advanced_keys {
            for ak in ak_list {
                self.write_advanced_key(ak)?;
                writes += 1;
            }
            // Re-apply actuation to bound keys since firmware resets them on RS/SOCD write.
            let _ = self.write_actuation_bulk(&desired_act, 0, flash_op);
        }

        if flash_op == FlashOp::EraseAndWrite {
            let _ = self.save_lighting();
        }

        let _ = (key_str, DEFAULT_LED_GAIN);
        Ok(writes)
    }
}
