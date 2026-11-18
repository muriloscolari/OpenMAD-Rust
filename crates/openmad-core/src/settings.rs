//! Application-level settings (update check, start at login, LED channel gain).

use crate::protocol::DEFAULT_LED_GAIN;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_true")]
    pub check_updates: bool,
    #[serde(default)]
    pub start_at_login: bool,
    #[serde(default = "default_gain")]
    pub led_gain: [f32; 3],
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_true() -> bool {
    true
}

fn default_gain() -> [f32; 3] {
    DEFAULT_LED_GAIN
}

fn default_language() -> String {
    "pt".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            check_updates: true,
            start_at_login: get_start_at_login(),
            led_gain: DEFAULT_LED_GAIN,
            language: default_language(),
        }
    }
}

impl AppSettings {
    pub fn load(path: &Path) -> Self {
        if let Ok(text) = std::fs::read_to_string(path) {
            let trimmed = text.trim_start_matches('\u{feff}');
            if let Ok(mut s) = serde_json::from_str::<Self>(trimmed) {
                let is_legacy_dim_gain = (s.led_gain[0] - 1.0).abs() < 0.005
                    && (s.led_gain[1] - 0.23).abs() < 0.005
                    && (s.led_gain[2] - 0.27).abs() < 0.005;
                if is_legacy_dim_gain {
                    s.led_gain = DEFAULT_LED_GAIN;
                    let _ = s.save(path);
                }
                s.start_at_login = get_start_at_login();
                return s;
            }
        }
        Self::default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, json)
    }
}

#[cfg(target_os = "linux")]
fn autostart_desktop_path() -> Option<std::path::PathBuf> {
    let cfg_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
        })?;
    Some(cfg_home.join("autostart/openmad.desktop"))
}

#[cfg(target_os = "linux")]
pub fn get_start_at_login() -> bool {
    autostart_desktop_path()
        .map(|p| p.is_file())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
pub fn set_start_at_login(enabled: bool) -> bool {
    let Some(path) = autostart_desktop_path() else {
        return false;
    };
    if enabled {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let exe = std::env::current_exe()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "openmad".to_string());
        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=OpenMAD\n\
             Comment=OpenMAD-Rust driver (minimized to tray)\n\
             Exec=\"{exe}\" --minimized\n\
             Icon=openmad-rust\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n"
        );
        let _ = std::fs::write(&path, content);
    } else if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    get_start_at_login()
}

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const RUN_VAL: &str = "OpenMAD";

#[cfg(target_os = "windows")]
pub fn get_start_at_login() -> bool {
    std::process::Command::new("reg")
        .args(["query", RUN_KEY, "/v", RUN_VAL])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub fn set_start_at_login(enabled: bool) -> bool {
    if enabled {
        let exe = std::env::current_exe()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "openmad.exe".to_string());
        let cmd = format!("\"{exe}\" --minimized");
        let _ = std::process::Command::new("reg")
            .args(["add", RUN_KEY, "/v", RUN_VAL, "/t", "REG_SZ", "/d", &cmd, "/f"])
            .output();
    } else {
        let _ = std::process::Command::new("reg")
            .args(["delete", RUN_KEY, "/v", RUN_VAL, "/f"])
            .output();
    }
    get_start_at_login()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn get_start_at_login() -> bool {
    false
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn set_start_at_login(_enabled: bool) -> bool {
    false
}
