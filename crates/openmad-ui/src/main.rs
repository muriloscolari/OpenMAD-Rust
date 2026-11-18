#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod i18n;
mod icon;
mod theme;
mod tray;
mod ui;
mod widgets;

use app::OpenMadApp;
use iced::Size;
use icon::generate_app_icon_rgba;
use openmad_core::device::Mad68;
use openmad_core::devices::UnlockedBoards;
use openmad_core::paths::{profile_dir, settings_path, unlocked_boards_path};
use openmad_core::profile::{list_profiles, profile_path, Profile};
use openmad_core::protocol::FlashOp;
use openmad_core::settings::AppSettings;
use theme::{
    app_theme, FONT_INTER, INTER_MEDIUM_BYTES, INTER_REGULAR_BYTES, INTER_SEMIBOLD_BYTES,
    LUCIDE_FONT_BYTES,
};

pub fn load_window_icon() -> Option<iced::window::Icon> {
    let rgba = generate_app_icon_rgba(256);
    iced::window::icon::from_rgba(rgba, 256, 256).ok()
}

#[cfg(target_os = "linux")]
fn install_linux_desktop_entry() {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        });
    let Some(data_dir) = data_home else {
        return;
    };

    let icon_dir = data_dir.join("icons/hicolor/256x256/apps");
    let icon_path_legacy = icon_dir.join("openmad.png");
    let icon_path = icon_dir.join("openmad-rust.png");
    let mut updated = false;
    let rgba = generate_app_icon_rgba(256);
    if let Some(buf) = image::RgbaImage::from_raw(256, 256, rgba) {
        let mut png_bytes = Vec::new();
        if image::DynamicImage::ImageRgba8(buf)
            .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .is_ok()
        {
            let _ = std::fs::create_dir_all(&icon_dir);
            for target in [&icon_path, &icon_path_legacy] {
                let needs_icon = std::fs::read(target)
                    .map(|cur| cur != png_bytes)
                    .unwrap_or(true);
                if needs_icon && std::fs::write(target, &png_bytes).is_ok() {
                    updated = true;
                }
            }
        }
    }

    let apps_dir = data_dir.join("applications");
    let desktop_path = apps_dir.join("openmad.desktop");
    let exe_str = std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "openmad".to_string());
    let icon_str = icon_path.to_string_lossy();
    let desktop_content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=OpenMAD\n\
         Comment=Local driver for MAD68 HE / MAD60 HE magnetic-switch keyboards\n\
         Exec=\"{exe_str}\"\n\
         Icon={icon_str}\n\
         Terminal=false\n\
         Categories=Utility;HardwareSettings;\n\
         StartupWMClass=openmad\n"
    );
    let needs_write = std::fs::read_to_string(&desktop_path)
        .map(|cur| cur != desktop_content)
        .unwrap_or(true);
    if needs_write
        && std::fs::create_dir_all(&apps_dir).is_ok()
        && std::fs::write(&desktop_path, desktop_content).is_ok()
    {
        updated = true;
    }

    if updated {
        let hicolor_dir = data_dir.join("icons/hicolor");
        let _ = std::process::Command::new("gtk-update-icon-cache")
            .args(["-f", "-t"])
            .arg(&hicolor_dir)
            .output();
        let _ = std::process::Command::new("kbuildsycoca6").output();
    }
}

pub fn window_settings() -> iced::window::Settings {
    #[allow(unused_mut)]
    let mut settings = iced::window::Settings {
        size: Size::new(1240.0, 820.0),
        icon: load_window_icon(),
        exit_on_close_request: false,
        ..Default::default()
    };
    #[cfg(target_os = "linux")]
    {
        settings.platform_specific = iced::window::settings::PlatformSpecific {
            application_id: "openmad".to_string(),
            ..Default::default()
        };
    }
    settings
}

fn try_cli_command() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "OpenMAD-Rust v{}\n\
             Usage: openmad [OPTIONS]\n\n\
             Options:\n  \
               --minimized, --tray          Start minimized to system tray\n  \
               --list-profiles              Print saved profile names and exit\n  \
               --apply-profile <NAME>       Apply a profile to RAM and exit\n  \
               --screenshot=<SCREEN>        Capture UI screenshot and exit",
            env!("CARGO_PKG_VERSION")
        );
        return Some(0);
    }
    if args.iter().any(|a| a == "--list-profiles") {
        for p in list_profiles(&profile_dir()) {
            println!("{p}");
        }
        return Some(0);
    }
    let apply_name = args
        .iter()
        .find_map(|a| a.strip_prefix("--apply-profile=").map(str::to_string))
        .or_else(|| {
            args.windows(2)
                .find(|w| w[0] == "--apply-profile")
                .map(|w| w[1].clone())
        });
    if let Some(name) = apply_name {
        let path = match profile_path(&profile_dir(), &name) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return Some(1);
            }
        };
        let prof = match Profile::load(&path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return Some(1);
            }
        };
        let settings = AppSettings::load(&settings_path());
        let unlocked = UnlockedBoards::load(&unlocked_boards_path());
        let mut kb = match Mad68::open(true, false, settings.led_gain, &unlocked) {
            Ok(kb) => kb,
            Err(e) => {
                eprintln!("Failed to open keyboard: {e}");
                return Some(1);
            }
        };
        match kb.apply_profile(&prof, FlashOp::Normal) {
            Ok(writes) => {
                println!("Applied profile '{name}' ({writes} packet group(s))");
                return Some(0);
            }
            Err(e) => {
                eprintln!("Failed to apply profile '{name}': {e}");
                return Some(1);
            }
        }
    }
    None
}

fn main() -> iced::Result {
    if let Some(code) = try_cli_command() {
        std::process::exit(code);
    }

    #[cfg(target_os = "linux")]
    install_linux_desktop_entry();

    iced::daemon(OpenMadApp::title, OpenMadApp::update, OpenMadApp::view)
        .font(INTER_REGULAR_BYTES)
        .font(INTER_MEDIUM_BYTES)
        .font(INTER_SEMIBOLD_BYTES)
        .font(LUCIDE_FONT_BYTES)
        .default_font(FONT_INTER)
        .theme(|_, _| app_theme())
        .subscription(OpenMadApp::subscription)
        .run_with(OpenMadApp::new)
}
