//! Named, file-based configuration profiles (100% compatible with OpenMAD JSON).

use crate::features::{
    AdvancedKey, BoxLight, DeadBand, Dks, DksAction, KeyboardFeature, LightInfo, RapidTrigger,
    TapDance,
};
use crate::keycodes::{decode_all_layers, encode_all_layers, keymap_diff_count};
use crate::protocol::AdvancedKeyMode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const PROFILE_VERSION: u32 = 2;
pub const ADVANCED_KEY_SLOTS: usize = 16;
pub const PER_KEY_EFFECT: u8 = 1;
pub const COLOR_TOLERANCE: i16 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TriggerBinding {
    #[serde(default)]
    pub exe: String,
    #[serde(default)]
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RapidTriggerSpec {
    pub enabled: bool,
    pub press_mm: f32,
    pub release_mm: f32,
}

impl From<&RapidTrigger> for RapidTriggerSpec {
    fn from(rt: &RapidTrigger) -> Self {
        Self {
            enabled: rt.enabled,
            press_mm: rt.press_mm,
            release_mm: rt.release_mm,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default = "default_version")]
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub triggers: Vec<TriggerBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_exe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_title: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuation_default: Option<f32>,
    #[serde(default)]
    pub actuation_overrides: BTreeMap<String, f32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rapid_trigger_default: Option<RapidTriggerSpec>,
    #[serde(default)]
    pub rapid_trigger_overrides: BTreeMap<String, RapidTriggerSpec>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dead_band: Option<DeadBand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<KeyboardFeature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_mode: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom_optimize: Option<u8>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light: Option<LightInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub box_light: Option<BoxLight>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_colors: Option<Vec<[u8; 3]>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dks: Option<Vec<Dks>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tap_dance: Option<Vec<TapDance>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced_keys: Option<Vec<AdvancedKey>>,

    /// 4 layers x rows x cols of 16-bit QMK keycodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keymap: Option<Vec<Vec<Vec<u16>>>>,
}

fn default_version() -> u32 {
    PROFILE_VERSION
}

impl Profile {
    pub fn blank(name: impl Into<String>) -> Self {
        Self {
            version: PROFILE_VERSION,
            name: name.into(),
            description: String::new(),
            triggers: Vec::new(),
            trigger_exe: None,
            trigger_title: None,
            actuation_default: Some(2.0),
            actuation_overrides: BTreeMap::new(),
            rapid_trigger_default: Some(RapidTriggerSpec {
                enabled: false,
                press_mm: 0.30,
                release_mm: 0.30,
            }),
            rapid_trigger_overrides: BTreeMap::new(),
            dead_band: None,
            feature: None,
            game_mode: None,
            bottom_optimize: None,
            light: None,
            box_light: None,
            key_colors: None,
            dks: None,
            tap_dance: None,
            advanced_keys: None,
            keymap: None,
        }
    }

    pub fn effective_triggers(&self) -> Vec<TriggerBinding> {
        if !self.triggers.is_empty() {
            return self
                .triggers
                .iter()
                .filter(|t| !t.exe.trim().is_empty() || !t.title.trim().is_empty())
                .cloned()
                .collect();
        }
        let exe = self.trigger_exe.as_deref().unwrap_or("").trim();
        let title = self.trigger_title.as_deref().unwrap_or("").trim();
        if !exe.is_empty() || !title.is_empty() {
            vec![TriggerBinding {
                exe: exe.to_string(),
                title: title.to_string(),
            }]
        } else {
            Vec::new()
        }
    }

    pub fn actuation_for(&self, row: usize, col: usize) -> f32 {
        let key = key_str(row, col);
        self.actuation_overrides
            .get(&key)
            .copied()
            .or(self.actuation_default)
            .unwrap_or(2.0)
    }

    pub fn rapid_trigger_for(&self, row: usize, col: usize) -> RapidTriggerSpec {
        let key = key_str(row, col);
        self.rapid_trigger_overrides
            .get(&key)
            .copied()
            .or(self.rapid_trigger_default)
            .unwrap_or(RapidTriggerSpec {
                enabled: false,
                press_mm: 0.30,
                release_mm: 0.30,
            })
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        Self::from_json_str(&text).map_err(|e| format!("Failed to parse {}: {e}", path.display()))
    }

    pub fn from_json_str(text: &str) -> Result<Self, String> {
        let trimmed = text.trim_start_matches('\u{feff}');
        if let Ok(prof) = serde_json::from_str::<Self>(trimmed) {
            return Ok(prof);
        }
        parse_legacy_python_profile(trimmed)
    }

    pub fn to_keymap_bytes(&self, layers: usize, rows: usize, cols: usize) -> Option<Vec<u8>> {
        let km = self.keymap.as_ref()?;
        if km.len() != layers
            || km.iter().any(|l| l.len() != rows || l.iter().any(|r| r.len() != cols))
        {
            return None;
        }
        Some(encode_all_layers(km, layers, rows, cols))
    }

    pub fn set_keymap_bytes(&mut self, bytes: &[u8], layers: usize, rows: usize, cols: usize) {
        if bytes.len() == layers * rows * cols * 2 {
            self.keymap = Some(decode_all_layers(bytes, layers, rows, cols));
        }
    }

    /// Returns `None` if this profile has no keymap or mismatched dimensions,
    /// otherwise `Some(differing_key_count)` against `live_keymap`.
    pub fn keymap_diff_against(
        &self,
        live_keymap: &[u8],
        layers: usize,
        rows: usize,
        cols: usize,
    ) -> Option<usize> {
        let want = self.to_keymap_bytes(layers, rows, cols)?;
        if want.len() != live_keymap.len() {
            return None;
        }
        Some(keymap_diff_count(&want, live_keymap))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize profile: {e}"))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))
    }
}

pub fn key_str(row: usize, col: usize) -> String {
    format!("{row},{col}")
}

pub fn safe_profile_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err(format!("Invalid profile name: {name:?}"));
    }
    Ok(trimmed.to_string())
}

pub fn profile_path(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let clean = safe_profile_name(name)?;
    Ok(dir.join(format!("{clean}.json")))
}

pub fn list_profiles(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                p.file_stem().and_then(|s| s.to_str()).map(str::to_string)
            } else {
                None
            }
        })
        .collect();
    names.sort_by_key(|a| a.to_lowercase());
    names
}

pub fn fit_key_colors(colors: &[[u8; 3]], rows: usize, cols: usize) -> Vec<[u8; 3]> {
    let total = rows * cols;
    let src_cols = if rows > 0 && colors.len().is_multiple_of(rows) && !colors.is_empty() {
        colors.len() / rows
    } else {
        cols
    };
    let mut out = Vec::with_capacity(total);
    if src_cols != cols {
        for r in 0..rows {
            for c in 0..cols {
                let idx = r * src_cols + c;
                if c < src_cols && idx < colors.len() {
                    out.push(colors[idx]);
                } else {
                    out.push([0, 0, 0]);
                }
            }
        }
    } else {
        out.extend_from_slice(colors);
    }
    out.resize(total, [0, 0, 0]);
    out
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OnboardRecord {
    #[serde(default)]
    pub name: String,
}

pub fn read_onboard_name(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let rec: OnboardRecord = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    if rec.name.trim().is_empty() {
        None
    } else {
        Some(rec.name)
    }
}

pub fn write_onboard_name(path: &Path, name: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let rec = OnboardRecord {
        name: name.to_string(),
    };
    let json = serde_json::to_string_pretty(&rec).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct LegacyProfileDoc {
    profile_version: u32,
    name: String,
    #[serde(default)]
    description: String,
    actuation: LegacyActuation,
    rapid_trigger: LegacyRapidTrigger,
    #[serde(default)]
    keymap_hex: Option<String>,
    #[serde(default)]
    key_colors: Option<Vec<[u8; 3]>>,
    #[serde(default)]
    light: Option<LightInfo>,
    #[serde(default)]
    box_light: Option<BoxLight>,
    #[serde(default)]
    performance: Option<serde_json::Value>,
    #[serde(default)]
    tap_dance: Option<Vec<TapDance>>,
    #[serde(default)]
    dks: Option<Vec<LegacyDks>>,
    #[serde(default)]
    advanced_keys: Option<Vec<LegacyAdvancedKey>>,
    #[serde(default)]
    triggers: Option<Vec<TriggerBinding>>,
    #[serde(default)]
    trigger_exe: Option<String>,
    #[serde(default)]
    trigger_title: Option<String>,
}

#[derive(Deserialize)]
struct LegacyActuation {
    default_mm: f32,
    #[serde(default)]
    overrides_mm: BTreeMap<String, f32>,
}

#[derive(Deserialize)]
struct LegacyRapidTrigger {
    default: RapidTriggerSpec,
    #[serde(default)]
    overrides: BTreeMap<String, RapidTriggerSpec>,
}

#[derive(Deserialize)]
struct LegacyDks {
    index: usize,
    #[serde(default)]
    actions: Vec<LegacyDksAction>,
}

#[derive(Deserialize)]
struct LegacyDksAction {
    #[serde(default)]
    status: u16,
    #[serde(default)]
    keycode: u16,
}

#[derive(Deserialize)]
struct LegacyAdvancedKey {
    index: usize,
    mode: serde_json::Value,
    #[serde(default)]
    id: u8,
    #[serde(default)]
    rs_apc_lv: u16,
    #[serde(default)]
    gapc_sw: u8,
    #[serde(default)]
    rt_sw: u8,
    #[serde(default)]
    key1_row: u8,
    #[serde(default)]
    key1_col: u8,
    #[serde(default)]
    key2_row: u8,
    #[serde(default)]
    key2_col: u8,
    #[serde(default)]
    layer: u8,
}

fn decode_hex_bytes(hex: &str) -> Option<Vec<u8>> {
    let clean: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if !clean.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(clean.len() / 2);
    for i in (0..clean.len()).step_by(2) {
        out.push(u8::from_str_radix(&clean[i..i + 2], 16).ok()?);
    }
    Some(out)
}

fn parse_legacy_python_profile(trimmed: &str) -> Result<Profile, String> {
    let leg: LegacyProfileDoc =
        serde_json::from_str(trimmed).map_err(|e| format!("Invalid profile JSON: {e}"))?;
    let _ = leg.profile_version;

    let mut prof = Profile::blank(leg.name);
    prof.description = leg.description;
    prof.actuation_default = Some(leg.actuation.default_mm);
    prof.actuation_overrides = leg.actuation.overrides_mm;
    prof.rapid_trigger_default = Some(leg.rapid_trigger.default);
    prof.rapid_trigger_overrides = leg.rapid_trigger.overrides;
    prof.light = leg.light;
    prof.box_light = leg.box_light;
    prof.key_colors = leg.key_colors;
    prof.tap_dance = leg.tap_dance;
    prof.triggers = leg.triggers.unwrap_or_default();
    prof.trigger_exe = leg.trigger_exe;
    prof.trigger_title = leg.trigger_title;

    if let Some(perf) = leg.performance {
        let mut feat = KeyboardFeature::default();
        let mut has_feat = false;
        if let Some(v) = perf.get("rgb_area").and_then(|x| x.as_u64()) {
            feat.rgb_area = v as u8;
            has_feat = true;
        }
        if let Some(v) = perf.get("wasd_switch").and_then(|x| x.as_u64()) {
            feat.wasd_switch = v as u8;
            has_feat = true;
        }
        if let Some(v) = perf.get("mac_switch").and_then(|x| x.as_u64()) {
            feat.mac_switch = v as u8;
            has_feat = true;
        }
        if let Some(v) = perf.get("win_lock").and_then(|x| x.as_u64()) {
            feat.win_lock = v as u8;
            has_feat = true;
        }
        if let Some(v) = perf.get("nkro_switch").and_then(|x| x.as_u64()) {
            feat.nkro_switch = v as u8;
            has_feat = true;
        }
        if has_feat {
            prof.feature = Some(feat);
        }
        if let Some(v) = perf.get("game_mode").and_then(|x| x.as_u64()) {
            prof.game_mode = Some(v as u8);
        }
        if let Some(v) = perf.get("bottom_optimize").and_then(|x| x.as_u64()) {
            prof.bottom_optimize = Some(v as u8);
        }
        let db_top = perf.get("dead_band_top").and_then(|x| x.as_f64());
        let db_bot = perf.get("dead_band_bottom").and_then(|x| x.as_f64());
        if db_top.is_some() || db_bot.is_some() {
            prof.dead_band = Some(DeadBand {
                top_mm: db_top.unwrap_or(0.0) as f32,
                bottom_mm: db_bot.unwrap_or(0.0) as f32,
            });
        }
    }

    if let Some(dks_list) = leg.dks {
        prof.dks = Some(
            dks_list
                .into_iter()
                .map(|d| {
                    let mut out = Dks::empty(d.index);
                    for (i, a) in d.actions.into_iter().take(4).enumerate() {
                        out.actions[i] = DksAction::from_keycode(a.keycode, a.status);
                    }
                    out
                })
                .collect(),
        );
    }

    if let Some(ak_list) = leg.advanced_keys {
        prof.advanced_keys = Some(
            ak_list
                .into_iter()
                .map(|a| {
                    let mode = match &a.mode {
                        serde_json::Value::String(s) => AdvancedKeyMode::parse_str(s),
                        serde_json::Value::Number(n) => {
                            AdvancedKeyMode::from_u8(n.as_u64().unwrap_or(0) as u8)
                        }
                        _ => AdvancedKeyMode::None,
                    };
                    AdvancedKey {
                        index: a.index,
                        mode,
                        id: a.id,
                        rs_apc_lv: a.rs_apc_lv,
                        gapc_sw: a.gapc_sw,
                        rt_sw: a.rt_sw,
                        key1_row: a.key1_row,
                        key1_col: a.key1_col,
                        key2_row: a.key2_row,
                        key2_col: a.key2_col,
                        layer: a.layer,
                    }
                })
                .collect(),
        );
    }

    if let Some(hex) = leg.keymap_hex {
        if let Some(bytes) = decode_hex_bytes(&hex) {
            let layers = 4usize;
            let rows = 5usize;
            if bytes.len() == layers * rows * 15 * 2 {
                prof.keymap = Some(decode_all_layers(&bytes, layers, rows, 15));
            } else if bytes.len() == layers * rows * 14 * 2 {
                prof.keymap = Some(decode_all_layers(&bytes, layers, rows, 14));
            } else if bytes.len() == layers * rows * 6 * 2 {
                prof.keymap = Some(decode_all_layers(&bytes, layers, rows, 6));
            }
        }
    }

    Ok(prof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_roundtrip_and_fit() {
        let mut p = Profile::blank("Gaming");
        p.actuation_default = Some(0.8);
        p.actuation_overrides.insert("2,1".into(), 0.4);
        p.triggers.push(TriggerBinding {
            exe: "cs2".into(),
            title: String::new(),
        });
        let json = serde_json::to_string_pretty(&p).unwrap();
        let parsed = Profile::from_json_str(&json).unwrap();
        assert_eq!(parsed.name, "Gaming");
        assert!((parsed.actuation_for(2, 1) - 0.4).abs() < 0.001);
        assert!((parsed.actuation_for(0, 0) - 0.8).abs() < 0.001);
        assert_eq!(parsed.effective_triggers().len(), 1);

        let colors68 = vec![[255, 0, 0]; 5 * 15];
        let fitted60 = fit_key_colors(&colors68, 5, 14);
        assert_eq!(fitted60.len(), 5 * 14);
        assert_eq!(fitted60[0], [255, 0, 0]);
    }

    #[test]
    fn test_legacy_python_profile_import() {
        let legacy = r#"{
            "profile_version": 2,
            "name": "CS2 Legacy",
            "description": "Imported from Python OpenMAD",
            "actuation": {
                "default_mm": 1.6,
                "overrides_mm": { "2,1": 0.35 }
            },
            "rapid_trigger": {
                "default": { "enabled": true, "press_mm": 0.2, "release_mm": 0.2 },
                "overrides": {}
            },
            "performance": {
                "win_lock": 1,
                "game_mode": 1,
                "dead_band_top": 0.10,
                "dead_band_bottom": 0.15
            },
            "advanced_keys": [
                {
                    "index": 0,
                    "mode": "SOCD_BALANCE",
                    "rs_apc_lv": 30,
                    "rt_sw": 1,
                    "key1_row": 2, "key1_col": 1,
                    "key2_row": 2, "key2_col": 3
                }
            ],
            "dks": [
                { "index": 0, "actions": [{ "status": 3, "keycode": 4 }] }
            ],
            "trigger_exe": "cs2"
        }"#;
        let p = Profile::from_json_str(legacy).unwrap();
        assert_eq!(p.name, "CS2 Legacy");
        assert!((p.actuation_for(2, 1) - 0.35).abs() < 0.001);
        assert!(p.rapid_trigger_for(0, 0).enabled);
        assert_eq!(p.feature.unwrap().win_lock, 1);
        assert_eq!(p.game_mode, Some(1));
        assert!((p.dead_band.unwrap().bottom_mm - 0.15).abs() < 0.001);
        assert_eq!(p.advanced_keys.as_ref().unwrap()[0].mode, AdvancedKeyMode::SocdBalance);
        assert_eq!(p.dks.as_ref().unwrap()[0].actions[0].keycode(), 4);
        assert_eq!(p.effective_triggers()[0].exe, "cs2");
    }

    #[test]
    fn test_safe_profile_name_and_keymap_roundtrip() {
        assert!(safe_profile_name("../evil").is_err());
        assert!(safe_profile_name("a/b").is_err());
        assert_eq!(safe_profile_name("  Gaming  ").unwrap(), "Gaming");

        let mut p = Profile::blank("KM");
        let mut raw = vec![0u8; 4 * 5 * 14 * 2];
        raw[0] = 0x00;
        raw[1] = 0x29;
        p.set_keymap_bytes(&raw, 4, 5, 14);
        assert_eq!(p.keymap_diff_against(&raw, 4, 5, 14), Some(0));
        raw[1] = 0x04;
        assert_eq!(p.keymap_diff_against(&raw, 4, 5, 14), Some(1));
    }
}
