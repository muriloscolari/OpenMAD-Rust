//! Named, file-based configuration profiles (100% compatible with OpenMAD JSON).

use crate::features::{
    AdvancedKey, BoxLight, DeadBand, Dks, KeyboardFeature, LightInfo, RapidTrigger, TapDance,
};
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
        let trimmed = text.trim_start_matches('\u{feff}');
        serde_json::from_str::<Self>(trimmed)
            .map_err(|e| format!("Failed to parse {}: {e}", path.display()))
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
    let src_cols = if rows > 0 && colors.len() % rows == 0 && !colors.is_empty() {
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
        let parsed: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Gaming");
        assert!((parsed.actuation_for(2, 1) - 0.4).abs() < 0.001);
        assert!((parsed.actuation_for(0, 0) - 0.8).abs() < 0.001);
        assert_eq!(parsed.effective_triggers().len(), 1);

        let colors68 = vec![[255, 0, 0]; 5 * 15];
        let fitted60 = fit_key_colors(&colors68, 5, 14);
        assert_eq!(fitted60.len(), 5 * 14);
        assert_eq!(fitted60[0], [255, 0, 0]);
    }
}
