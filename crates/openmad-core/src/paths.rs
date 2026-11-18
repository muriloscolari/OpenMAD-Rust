//! Cross-platform data and configuration paths for OpenMAD-Rust.

use directories::ProjectDirs;
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    // If running from a source checkout with a local `profiles` directory,
    // keep using the repo working tree for convenience during development.
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("Cargo.toml").is_file() && cwd.join("profiles").is_dir() {
            return cwd;
        }
    }
    if let Some(proj) = ProjectDirs::from("", "", "OpenMAD") {
        return proj.data_local_dir().to_path_buf();
    }
    PathBuf::from(".")
}

pub fn profile_dir() -> PathBuf {
    data_dir().join("profiles")
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn switcher_config_path() -> PathBuf {
    data_dir().join("switcher.json")
}

pub fn onboard_path() -> PathBuf {
    data_dir().join("onboard.json")
}

pub fn unlocked_boards_path() -> PathBuf {
    data_dir().join("unlocked.json")
}

pub fn led_gain_path() -> PathBuf {
    data_dir().join("led_gain.json")
}
