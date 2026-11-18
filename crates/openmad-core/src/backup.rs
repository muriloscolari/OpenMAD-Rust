//! Full onboard state backup and restore (JSON).

use crate::device::{FullConfig, Mad68, Result};
use crate::protocol::FlashOp;
use std::path::Path;

pub fn export_backup(kb: &mut Mad68, path: &Path) -> Result<FullConfig> {
    let cfg = kb.read_full_config()?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(&cfg).unwrap_or_default();
    std::fs::write(path, json).map_err(|e| crate::device::DeviceError::Protocol(e.to_string()))?;
    Ok(cfg)
}

pub fn restore_backup(kb: &mut Mad68, path: &Path, commit_flash: bool) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| crate::device::DeviceError::Protocol(e.to_string()))?;
    let cfg: FullConfig = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| crate::device::DeviceError::Protocol(e.to_string()))?;

    let flash_op = if commit_flash {
        FlashOp::EraseAndWrite
    } else {
        FlashOp::Normal
    };

    if !cfg.actuation.is_empty() {
        kb.write_actuation_bulk(&cfg.actuation, 0, flash_op)?;
    }
    if !cfg.rapid_trigger.is_empty() {
        kb.write_rapid_trigger_bulk(&cfg.rapid_trigger, 0, flash_op)?;
    }
    kb.write_dead_band(&cfg.dead_band)?;
    kb.write_feature(&cfg.feature)?;
    kb.write_game_mode(cfg.game_mode)?;
    kb.write_bottom_optimize(cfg.bottom_optimize)?;
    kb.write_light_info(&cfg.light)?;
    kb.write_box_light(&cfg.box_light)?;
    if !cfg.key_colors.is_empty() {
        kb.write_all_key_colors(&cfg.key_colors)?;
    }
    for d in &cfg.dks {
        kb.write_dks(d)?;
    }
    for td in &cfg.tap_dance {
        kb.write_tap_dance(td)?;
    }
    for ak in &cfg.advanced_keys {
        kb.write_advanced_key(ak)?;
    }
    if !cfg.keymap.is_empty() && cfg.keymap.len() == kb.keymap_size() {
        kb.write_keymap(&cfg.keymap)?;
    }
    for (idx, steps) in cfg.macros.iter().enumerate() {
        kb.write_macro_slot(idx, steps)?;
    }
    if commit_flash {
        kb.save_lighting()?;
    }
    Ok(())
}
