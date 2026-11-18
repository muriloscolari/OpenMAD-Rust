//! Wire protocol for the DuckBread onboard configuration interface.
//!
//! The keyboard speaks the VIA raw-HID protocol (32-byte reports, byte 0 is
//! the command ID) plus a vendor "custom channel" `0x96` carrying Hall-effect
//! features: per-key actuation, rapid trigger, dead band, DKS, SOCD/RS,
//! calibration, and IAP.

use std::fmt;

pub const VENDOR_ID: u16 = 0x373B;
pub const PRODUCT_ID: u16 = 0x1058;
pub const BOOTLOADER_PRODUCT_ID: u16 = 0x2024;

/// Protocol version this driver was reverse-engineered against and confirmed
/// on hardware.
pub const KNOWN_PROTOCOL_VERSION: u16 = 9;

/// Raw-HID config interface (QMK/VIA convention).
pub const USAGE_PAGE: u16 = 0xFF60;
pub const USAGE: u16 = 0x61;

pub const REPORT_SIZE: usize = 0x20; // 32 bytes
pub const PAYLOAD_OFFSET: usize = 0x01;
pub const MAX_PAYLOAD: usize = REPORT_SIZE - PAYLOAD_OFFSET; // 31

pub const BUFFER_HEADER: usize = 3;
pub const MAX_BUFFER_CHUNK: usize = MAX_PAYLOAD - BUFFER_HEADER; // 28 (0x1C)

pub const CUSTOM_CHANNEL: u8 = 0x96;
pub const DATA_REPORTING_PREFIX: [u8; 3] = [0x02, CUSTOM_CHANNEL, 0x14];

pub const BULK_OFFSET_AT: usize = 5;
pub const BULK_SIZE_AT: usize = 7;
pub const BULK_READ_DATA_AT: usize = 8;
pub const BULK_FLASH_OP_AT: usize = 8;
pub const BULK_WRITE_DATA_AT: usize = 9;

pub const ACTUATION_ENTRY_V2: usize = 2;
pub const RAPID_TRIGGER_ENTRY_V2: usize = 5;

pub const MAX_ACTUATION_READ: usize = (REPORT_SIZE - BULK_READ_DATA_AT) / ACTUATION_ENTRY_V2;
pub const MAX_ACTUATION_WRITE: usize = (REPORT_SIZE - BULK_WRITE_DATA_AT) / ACTUATION_ENTRY_V2;
pub const MAX_RT_READ: usize = (REPORT_SIZE - BULK_READ_DATA_AT) / RAPID_TRIGGER_ENTRY_V2;
pub const MAX_RT_WRITE: usize = (REPORT_SIZE - BULK_WRITE_DATA_AT) / RAPID_TRIGGER_ENTRY_V2;

pub const LAYER_COUNT: usize = 4;
pub const MATRIX_ROWS: usize = 5;
pub const MATRIX_COLS: usize = 15;
pub const KEYCODE_SIZE: usize = 2;
pub const KEYMAP_SIZE: usize = LAYER_COUNT * MATRIX_ROWS * MATRIX_COLS * KEYCODE_SIZE; // 600
pub const TOTAL_KEYS: usize = MATRIX_ROWS * MATRIX_COLS; // 75

/// Highest brightness the firmware accepts in a LightInfo write.
pub const BRIGHTNESS_MAX: u8 = 210;

pub const DEFAULT_EFFECT_SPEED: u8 = 128;
pub const DEFAULT_EFFECT_COLOUR: (u8, u8, u8) = (255, 255, 255);

pub const ANIMATED_EFFECTS: &[u8] = &[6, 7, 8, 9, 11, 13, 19, 23, 26, 28, 31, 36, 39];
pub const COLOUR_EFFECTS: &[u8] = &[2, 3, 4, 5, 6, 7, 8, 9, 11, 26, 28, 31, 36];

pub const TRAVEL_UNIT_MM: f32 = 0.01;
pub const TRAVEL_MAX_RAW: u16 = 0xFFFF;

pub const DEFAULT_LED_GAIN: [f32; 3] = [1.0, 1.0, 1.0];

/// VIA command IDs (byte 0).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cmd {
    GetProtocolVersion = 0x01,
    GetKeyboardValue = 0x02,
    SetKeyboardValue = 0x03,
    DynamicKeymapGetKeycode = 0x04,
    DynamicKeymapSetKeycode = 0x05,
    DynamicKeymapReset = 0x06,
    CustomSetValue = 0x07,
    CustomGetValue = 0x08,
    CustomSave = 0x09,
    EepromReset = 0x0A,
    BootloaderJump = 0x0B,
    DynamicKeymapMacroGetCount = 0x0C,
    DynamicKeymapMacroGetBufferSize = 0x0D,
    DynamicKeymapMacroGetBuffer = 0x0E,
    DynamicKeymapMacroSetBuffer = 0x0F,
    DynamicKeymapMacroReset = 0x10,
    DynamicKeymapGetLayerCount = 0x11,
    DynamicKeymapGetBuffer = 0x12,
    DynamicKeymapSetBuffer = 0x13,
    VialPrefix = 0xFE,
    Unhandled = 0xFF,
}

impl Cmd {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0x01 => Self::GetProtocolVersion,
            0x02 => Self::GetKeyboardValue,
            0x03 => Self::SetKeyboardValue,
            0x04 => Self::DynamicKeymapGetKeycode,
            0x05 => Self::DynamicKeymapSetKeycode,
            0x06 => Self::DynamicKeymapReset,
            0x07 => Self::CustomSetValue,
            0x08 => Self::CustomGetValue,
            0x09 => Self::CustomSave,
            0x0A => Self::EepromReset,
            0x0B => Self::BootloaderJump,
            0x0C => Self::DynamicKeymapMacroGetCount,
            0x0D => Self::DynamicKeymapMacroGetBufferSize,
            0x0E => Self::DynamicKeymapMacroGetBuffer,
            0x0F => Self::DynamicKeymapMacroSetBuffer,
            0x10 => Self::DynamicKeymapMacroReset,
            0x11 => Self::DynamicKeymapGetLayerCount,
            0x12 => Self::DynamicKeymapGetBuffer,
            0x13 => Self::DynamicKeymapSetBuffer,
            0xFE => Self::VialPrefix,
            0xFF => Self::Unhandled,
            _ => return None,
        })
    }

    pub fn is_write(self) -> bool {
        matches!(
            self,
            Self::SetKeyboardValue
                | Self::DynamicKeymapSetKeycode
                | Self::DynamicKeymapSetBuffer
                | Self::DynamicKeymapMacroSetBuffer
                | Self::CustomSetValue
                | Self::CustomSave
                | Self::DynamicKeymapReset
                | Self::DynamicKeymapMacroReset
                | Self::EepromReset
                | Self::BootloaderJump
        )
    }

    pub fn is_dangerous(self) -> bool {
        matches!(
            self,
            Self::DynamicKeymapReset
                | Self::DynamicKeymapMacroReset
                | Self::EepromReset
                | Self::BootloaderJump
        )
    }
}

/// Vendor sub-commands on channel `0x96`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vendor {
    TapDanceGet = 0x01,
    TapDanceSet = 0x02,
    OneTogTh = 0x03,
    OneRt = 0x07,
    Calibrate = 0x08,
    RealtimeAdcAxle = 0x09,
    RealtimeTripAxle = 0x0A,
    DeadBand = 0x0B,
    BufferTogTh = 0x0D, // also ENTRY_OP for Vial TapDance
    BufferRt = 0x0E,
    Dks = 0x0F,
    MixAxle = 0x10,
    Feature = 0x11,
    SaveLamplight = 0x12,
    Layer = 0x13,
    DataReporting = 0x14,
    ResetAll = 0x15,
    RealtimeAdcAxleBuffer = 0x16,
    RealtimeTripAxleBuffer = 0x17,
    CalibrationStart = 0x18,
    CalibrationFinish = 0x19,
    CompleteStatusBuffer = 0x1B,
    AdcTripCompStatusBuffer = 0x1C,
    BottomOptimizeSwitch = 0x1D,
    GameMode = 0x1E,
    Calibration = 0x1F,
    Rs = 0x20,
    BoxLight = 0x21,
    LightInfo = 0x41,
    SetCustomLamplight = 0x42,
    GetCustomLamplight = 0x45,
    Iap = 0x98,
}

impl Vendor {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0x01 => Self::TapDanceGet,
            0x02 => Self::TapDanceSet,
            0x03 => Self::OneTogTh,
            0x07 => Self::OneRt,
            0x08 => Self::Calibrate,
            0x09 => Self::RealtimeAdcAxle,
            0x0A => Self::RealtimeTripAxle,
            0x0B => Self::DeadBand,
            0x0D => Self::BufferTogTh,
            0x0E => Self::BufferRt,
            0x0F => Self::Dks,
            0x10 => Self::MixAxle,
            0x11 => Self::Feature,
            0x12 => Self::SaveLamplight,
            0x13 => Self::Layer,
            0x14 => Self::DataReporting,
            0x15 => Self::ResetAll,
            0x16 => Self::RealtimeAdcAxleBuffer,
            0x17 => Self::RealtimeTripAxleBuffer,
            0x18 => Self::CalibrationStart,
            0x19 => Self::CalibrationFinish,
            0x1B => Self::CompleteStatusBuffer,
            0x1C => Self::AdcTripCompStatusBuffer,
            0x1D => Self::BottomOptimizeSwitch,
            0x1E => Self::GameMode,
            0x1F => Self::Calibration,
            0x20 => Self::Rs,
            0x21 => Self::BoxLight,
            0x41 => Self::LightInfo,
            0x42 => Self::SetCustomLamplight,
            0x45 => Self::GetCustomLamplight,
            0x98 => Self::Iap,
            _ => return None,
        })
    }

    pub fn is_dangerous(self) -> bool {
        matches!(
            self,
            Self::Calibrate
                | Self::CalibrationStart
                | Self::CalibrationFinish
                | Self::ResetAll
                | Self::Iap
        )
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubField {
    Get = 0x01,
    GetBuffer = 0x02,
    Set = 0x03,
    SetBuffer = 0x04,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashOp {
    Normal = 0x00,
    Erase = 0x01,
    Write = 0x02,
    EraseAndWrite = 0x03,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardValue {
    Uptime = 0x01,
    LayoutOptions = 0x02,
    SwitchMatrixState = 0x03,
    FirmwareVersion = 0x04,
    DeviceIndication = 0x05,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AdvancedKeyMode {
    None = 0x00,
    Rs = 0x01,
    Socd = 0x02,
    SocdKey1 = 0x03,
    SocdKey2 = 0x04,
    SocdBalance = 0x05,
    Oks = 0x06,
}

impl AdvancedKeyMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0x01 => Self::Rs,
            0x02 => Self::Socd,
            0x03 => Self::SocdKey1,
            0x04 => Self::SocdKey2,
            0x05 => Self::SocdBalance,
            0x06 => Self::Oks,
            _ => Self::None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Rs => "RS",
            Self::Socd => "SOCD",
            Self::SocdKey1 => "SOCD_KEY1",
            Self::SocdKey2 => "SOCD_KEY2",
            Self::SocdBalance => "SOCD_BALANCE",
            Self::Oks => "OKS",
        }
    }

    pub fn parse_str(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "RS" => Self::Rs,
            "SOCD" => Self::Socd,
            "SOCD_KEY1" => Self::SocdKey1,
            "SOCD_KEY2" => Self::SocdKey2,
            "SOCD_BALANCE" => Self::SocdBalance,
            "OKS" => Self::Oks,
            _ => Self::None,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroAction {
    SsTapCode = 0x01,
    SsDownCode = 0x02,
    SsUpCode = 0x03,
    SsDelayCode = 0x04,
    VialMacroExtTap = 0x05,
    VialMacroExtDown = 0x06,
    VialMacroExtUp = 0x07,
}

impl MacroAction {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0x01 => Self::SsTapCode,
            0x02 => Self::SsDownCode,
            0x03 => Self::SsUpCode,
            0x04 => Self::SsDelayCode,
            0x05 => Self::VialMacroExtTap,
            0x06 => Self::VialMacroExtDown,
            0x07 => Self::VialMacroExtUp,
            _ => return None,
        })
    }
}

pub fn keymap_size(rows: usize, cols: usize, layers: usize) -> usize {
    layers * rows * cols * KEYCODE_SIZE
}

pub fn keymap_offset(layer: usize, row: usize, col: usize, rows: usize, cols: usize) -> usize {
    ((layer * rows + row) * cols + col) * KEYCODE_SIZE
}

fn apply_gain(value: u8, gain: f32) -> u8 {
    ((value as f32 * gain).round() as i32).clamp(0, 255) as u8
}

pub fn encode_rgb(r: u8, g: u8, b: u8, gain: [f32; 3]) -> [u8; 3] {
    [
        apply_gain(r, gain[0]),
        apply_gain(g, gain[1]),
        apply_gain(b, gain[2]),
    ]
}

pub fn decode_rgb(data: [u8; 3], gain: [f32; 3]) -> [u8; 3] {
    let mut out = [0u8; 3];
    for i in 0..3 {
        let g = gain[i];
        out[i] = if g > 0.0 {
            ((data[i] as f32 / g).round() as i32).clamp(0, 255) as u8
        } else {
            data[i]
        };
    }
    out
}

pub fn travel_to_mm(raw: u16) -> f32 {
    ((raw as f32 * TRAVEL_UNIT_MM) * 100.0).round() / 100.0
}

pub fn mm_to_travel(mm: f32) -> u16 {
    ((mm / TRAVEL_UNIT_MM).round() as i32).clamp(0, TRAVEL_MAX_RAW as i32) as u16
}

pub fn travel_bytes(mm: f32) -> [u8; 2] {
    mm_to_travel(mm).to_be_bytes()
}

pub fn travel_from_bytes(hi: u8, lo: u8) -> f32 {
    travel_to_mm(u16::from_be_bytes([hi, lo]))
}

pub fn build(cmd: Cmd, payload: &[u8]) -> Result<[u8; REPORT_SIZE], String> {
    if payload.len() > MAX_PAYLOAD {
        return Err(format!("payload {} exceeds {}", payload.len(), MAX_PAYLOAD));
    }
    let mut buf = [0u8; REPORT_SIZE];
    buf[0] = cmd as u8;
    buf[PAYLOAD_OFFSET..PAYLOAD_OFFSET + payload.len()].copy_from_slice(payload);
    Ok(buf)
}

pub fn build_vendor(cmd: Cmd, sub: Vendor, payload: &[u8]) -> Result<[u8; REPORT_SIZE], String> {
    let mut full = Vec::with_capacity(2 + payload.len());
    full.push(CUSTOM_CHANNEL);
    full.push(sub as u8);
    full.extend_from_slice(payload);
    build(cmd, &full)
}

pub fn build_buffer_read(cmd: Cmd, offset: u16, size: u8) -> Result<[u8; REPORT_SIZE], String> {
    if (size as usize) > MAX_BUFFER_CHUNK {
        return Err(format!("size {} exceeds {}", size, MAX_BUFFER_CHUNK));
    }
    let off = offset.to_be_bytes();
    build(cmd, &[off[0], off[1], size])
}

pub fn build_buffer_write(cmd: Cmd, offset: u16, data: &[u8]) -> Result<[u8; REPORT_SIZE], String> {
    if data.len() > MAX_BUFFER_CHUNK {
        return Err(format!("data {} exceeds {}", data.len(), MAX_BUFFER_CHUNK));
    }
    let off = offset.to_be_bytes();
    let mut payload = Vec::with_capacity(3 + data.len());
    payload.push(off[0]);
    payload.push(off[1]);
    payload.push(data.len() as u8);
    payload.extend_from_slice(data);
    build(cmd, &payload)
}

#[derive(Clone, PartialEq, Eq)]
pub struct Reply {
    pub raw: [u8; REPORT_SIZE],
}

impl Reply {
    pub fn from_slice(data: &[u8]) -> Self {
        let mut raw = [0u8; REPORT_SIZE];
        let n = data.len().min(REPORT_SIZE);
        raw[..n].copy_from_slice(&data[..n]);
        Self { raw }
    }

    pub fn command_id(&self) -> u8 {
        self.raw[0]
    }

    pub fn payload(&self) -> &[u8] {
        &self.raw[PAYLOAD_OFFSET..]
    }

    pub fn buffer_data(&self) -> &[u8] {
        let size = self.raw[PAYLOAD_OFFSET + 2] as usize;
        let start = PAYLOAD_OFFSET + BUFFER_HEADER;
        let end = (start + size).min(REPORT_SIZE);
        &self.raw[start..end]
    }

    pub fn is_unhandled(&self) -> bool {
        self.command_id() == Cmd::Unhandled as u8
    }

    pub fn is_data_reporting(&self) -> bool {
        self.raw[0..3] == DATA_REPORTING_PREFIX
    }

    pub fn stream_type(&self) -> Option<u8> {
        if self.is_data_reporting() {
            Some(self.raw[3])
        } else {
            None
        }
    }
}

impl fmt::Debug for Reply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Reply(cmd={:#04x}, data={:02x?})", self.raw[0], &self.raw[1..9])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_travel_conversion() {
        assert_eq!(mm_to_travel(1.50), 150);
        assert_eq!(travel_to_mm(150), 1.50);
        assert_eq!(travel_bytes(2.00), [0x00, 0xC8]);
        assert_eq!(travel_from_bytes(0x00, 0xC8), 2.00);
    }

    #[test]
    fn test_packet_builders() {
        let pkt = build_vendor(Cmd::GetKeyboardValue, Vendor::LightInfo, &[SubField::Get as u8]).unwrap();
        assert_eq!(pkt[0], 0x02);
        assert_eq!(pkt[1], 0x96);
        assert_eq!(pkt[2], 0x41);
        assert_eq!(pkt[3], 0x01);
        assert_eq!(pkt.len(), REPORT_SIZE);

        let buf_rd = build_buffer_read(Cmd::DynamicKeymapGetBuffer, 0x0120, 28).unwrap();
        assert_eq!(&buf_rd[0..4], &[0x12, 0x01, 0x20, 0x1C]);
    }

    #[test]
    fn test_keymap_offsets() {
        assert_eq!(keymap_size(5, 15, 4), 600);
        assert_eq!(keymap_size(5, 14, 4), 560);
        assert_eq!(keymap_offset(1, 2, 3, 5, 15), ((1 * 5 + 2) * 15 + 3) * 2);
    }
}
