//! Privacy-safe hardware diagnostic report and volatile write-probe helper.

use crate::device::{find_interfaces, HidInterfaceInfo, Mad68};
use crate::devices::{lookup, VENDOR_ID};
use crate::paths::reports_dir;
use crate::protocol::{FlashOp, KNOWN_PROTOCOL_VERSION};
use crate::updater::{APP_VERSION, REPO_SLUG};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticDevice {
    pub vendor_id: String,
    pub product_id: String,
    pub registry_name: String,
    pub in_registry: bool,
    pub verified: bool,
    pub rows: usize,
    pub cols: usize,
    pub layers: usize,
    pub hid_interfaces: Vec<HidInterfaceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticFirmware {
    pub protocol_version: Option<u16>,
    pub firmware_version: Option<u32>,
    pub layer_count: Option<usize>,
    pub macro_count: Option<usize>,
    pub macro_buffer_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticGeometry {
    pub actuation_len: usize,
    pub rapid_trigger_len: usize,
    pub key_colors_len: usize,
    pub populated_mask: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticProbeEntry {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteProbeResult {
    pub ran: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrote_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_back_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_ok: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restored_mm: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub report_version: u32,
    pub app_version: String,
    pub generated_unix_secs: u64,
    pub driver_known_protocol: u16,
    pub device: DiagnosticDevice,
    pub firmware: DiagnosticFirmware,
    pub geometry_readback: DiagnosticGeometry,
    pub probes: BTreeMap<String, DiagnosticProbeEntry>,
    pub write_probe: WriteProbeResult,
}

#[derive(Debug, Clone)]
pub struct DiagnosticOutput {
    pub path: PathBuf,
    pub report: DiagnosticReport,
    pub json: String,
    pub issue_url: String,
}

fn format_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

/// Runs a volatile (`FlashOp::Normal`) read -> write -> verify -> restore test
/// on key 0 without touching onboard flash endurance.
pub fn run_volatile_write_probe(kb: &mut Mad68) -> WriteProbeResult {
    let prev_writes = kb.writes;
    let prev_reason = kb.read_only_reason.clone();
    kb.writes = true;
    kb.read_only_reason = None;

    let mut result = WriteProbeResult {
        ran: true,
        original_mm: None,
        wrote_mm: None,
        read_back_mm: None,
        match_ok: None,
        restored_mm: None,
        restored: None,
        error: None,
    };

    let original = match kb.read_actuation_bulk() {
        Ok(vals) if !vals.is_empty() => vals[0],
        Ok(_) => {
            result.error = Some("empty actuation array".to_string());
            kb.writes = prev_writes;
            kb.read_only_reason = prev_reason;
            return result;
        }
        Err(e) => {
            result.error = Some(e.to_string());
            kb.writes = prev_writes;
            kb.read_only_reason = prev_reason;
            return result;
        }
    };
    result.original_mm = Some((original * 100.0).round() / 100.0);

    let target = if (original - 1.50).abs() > 0.10 { 1.50 } else { 2.20 };
    result.wrote_mm = Some(target);

    if let Err(e) = kb.write_actuation_bulk(&[target], 0, FlashOp::Normal) {
        result.error = Some(e.to_string());
        result.match_ok = Some(false);
    } else {
        match kb.read_actuation_bulk() {
            Ok(vals) if !vals.is_empty() => {
                let rb = vals[0];
                result.read_back_mm = Some((rb * 100.0).round() / 100.0);
                result.match_ok = Some((rb - target).abs() <= 0.05);
            }
            Ok(_) => {
                result.error = Some("empty readback".to_string());
                result.match_ok = Some(false);
            }
            Err(e) => {
                result.error = Some(e.to_string());
                result.match_ok = Some(false);
            }
        }
    }

    // Always restore original value in RAM
    match kb.write_actuation_bulk(&[original], 0, FlashOp::Normal) {
        Ok(()) => {
            if let Ok(vals) = kb.read_actuation_bulk() {
                if let Some(&restored_val) = vals.first() {
                    result.restored_mm = Some((restored_val * 100.0).round() / 100.0);
                    result.restored = Some((restored_val - original).abs() <= 0.05);
                }
            }
        }
        Err(e) => {
            result.restored = Some(false);
            if result.error.is_none() {
                result.error = Some(format!("restore failed: {e}"));
            }
        }
    }

    kb.writes = prev_writes;
    kb.read_only_reason = prev_reason;
    result
}

pub fn build_diagnostic_report(kb: &mut Mad68, write_probe: bool) -> DiagnosticReport {
    let interfaces = find_interfaces()
        .unwrap_or_default()
        .into_iter()
        .map(|mut info| {
            // Keep path minimal in case it contains device serial tokens on Windows/Linux
            info.path = if info.path.len() > 64 {
                format!("...{}", &info.path[info.path.len() - 48..])
            } else {
                info.path
            };
            info
        })
        .collect();

    let mut probes = BTreeMap::new();
    for (name, res) in kb.iter_probe_reads() {
        match res {
            Ok(bytes) => {
                probes.insert(
                    name,
                    DiagnosticProbeEntry {
                        ok: true,
                        hex: Some(format_hex(&bytes)),
                        error: None,
                    },
                );
            }
            Err(err) => {
                probes.insert(
                    name,
                    DiagnosticProbeEntry {
                        ok: false,
                        hex: None,
                        error: Some(err),
                    },
                );
            }
        }
    }

    let act_len = kb.read_actuation_bulk().map(|v| v.len()).unwrap_or(0);
    let rt_len = kb.read_rapid_trigger_bulk().map(|v| v.len()).unwrap_or(0);
    let kc_len = kb.read_all_key_colors().map(|v| v.len()).unwrap_or(0);
    let pop_str: String = kb
        .populated_mask()
        .into_iter()
        .map(|b| if b { '1' } else { '0' })
        .collect();

    let wp = if write_probe {
        run_volatile_write_probe(kb)
    } else {
        WriteProbeResult {
            ran: false,
            original_mm: None,
            wrote_mm: None,
            read_back_mm: None,
            match_ok: None,
            restored_mm: None,
            restored: None,
            error: None,
        }
    };

    let generated_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    DiagnosticReport {
        report_version: 1,
        app_version: APP_VERSION.to_string(),
        generated_unix_secs,
        driver_known_protocol: KNOWN_PROTOCOL_VERSION,
        device: DiagnosticDevice {
            vendor_id: format!("{VENDOR_ID:#06x}"),
            product_id: format!("{:#06x}", kb.spec.product_id),
            registry_name: kb.spec.name.clone(),
            in_registry: lookup(kb.spec.product_id).is_some(),
            verified: kb.spec.verified,
            rows: kb.spec.rows,
            cols: kb.spec.cols,
            layers: kb.spec.layers,
            hid_interfaces: interfaces,
        },
        firmware: DiagnosticFirmware {
            protocol_version: kb.protocol_version,
            firmware_version: kb.firmware_version,
            layer_count: kb.layer_count().ok(),
            macro_count: kb.macro_count().ok(),
            macro_buffer_size: kb.macro_buffer_size().ok(),
        },
        geometry_readback: DiagnosticGeometry {
            actuation_len: act_len,
            rapid_trigger_len: rt_len,
            key_colors_len: kc_len,
            populated_mask: pop_str,
        },
        probes,
        write_probe: wp,
    }
}

pub fn save_and_prepare_diagnostic(kb: &mut Mad68, write_probe: bool) -> Result<DiagnosticOutput, String> {
    let report = build_diagnostic_report(kb, write_probe);
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    let dir = reports_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let filename = format!(
        "openmad-report-{}-{}.json",
        report.device.product_id, report.generated_unix_secs
    );
    let path = dir.join(filename);
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;
    let issue_url = diagnostic_issue_url(&report);
    Ok(DiagnosticOutput {
        path,
        report,
        json,
        issue_url,
    })
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else if b == b' ' {
            out.push_str("%20");
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn diagnostic_issue_url(report: &DiagnosticReport) -> String {
    let title = format!(
        "Board report: {} ({})",
        report.device.registry_name, report.device.product_id
    );
    let wp_line = if !report.write_probe.ran {
        "not run (read-only)".to_string()
    } else if let Some(ref err) = report.write_probe.error {
        format!("FAILED - {err}")
    } else if report.write_probe.match_ok == Some(true) {
        format!(
            "PASSED - wrote {:?} mm, read back {:?} mm",
            report.write_probe.wrote_mm, report.write_probe.read_back_mm
        )
    } else {
        format!(
            "MISMATCH - wrote {:?} mm, read back {:?} mm",
            report.write_probe.wrote_mm, report.write_probe.read_back_mm
        )
    };

    let body = format!(
        "Board: **{}**\n\
         VID:PID: `{}:{}`\n\
         In registry: {} | Verified: {}\n\
         Matrix: {} x {}, {} layers\n\
         Protocol version: **{:?}** (known: {})\n\
         Readback lengths: actuation={}, rt={}, colours={}\n\
         Write test: {}\n\
         OpenMAD-Rust v{}\n\n\
         ---\n\n\
         Please attach the generated JSON file from your reports folder.",
        report.device.registry_name,
        report.device.vendor_id,
        report.device.product_id,
        report.device.in_registry,
        report.device.verified,
        report.device.rows,
        report.device.cols,
        report.device.layers,
        report.firmware.protocol_version,
        report.driver_known_protocol,
        report.geometry_readback.actuation_len,
        report.geometry_readback.rapid_trigger_len,
        report.geometry_readback.key_colors_len,
        wp_line,
        report.app_version,
    );

    format!(
        "https://github.com/{REPO_SLUG}/issues/new?title={}&body={}",
        percent_encode(&title),
        percent_encode(&body)
    )
}
