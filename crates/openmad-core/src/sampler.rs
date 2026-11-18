//! Background HID sampler, telemetry poller, and command job queue.

use crate::backup::{export_backup, restore_backup};
use crate::device::{find_interfaces, FullConfig, HidInterfaceInfo, Mad68};
use crate::devices::{default_board, BoardSpec, UnlockedBoards};
use crate::diagnostics::{save_and_prepare_diagnostic, DiagnosticOutput};
use crate::features::{
    AdvancedKey, BoxLight, DeadBand, Dks, KeyboardFeature, LightInfo, MacroStep, TapDance,
};
use crate::paths::{
    onboard_path, profile_dir, settings_path, switcher_config_path, unlocked_boards_path,
};
use crate::profile::{profile_path, read_onboard_name, write_onboard_name, Profile};
use crate::protocol::{travel_to_mm, FlashOp, KNOWN_PROTOCOL_VERSION};
use crate::settings::AppSettings;
use crate::switcher::{foreground_app, ForegroundApp, SwitcherConfig, SwitcherState};
use crossbeam_channel::{unbounded, Sender};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, RwLock,
};
use std::thread;
use std::time::{Duration, Instant};

const FOCUS_TRACE_LEN: usize = 160;
const FOCUS_BURST_SAMPLES: usize = 4;

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub connected: bool,
    pub in_bootloader: bool,
    pub board: BoardSpec,
    pub protocol_version: Option<u16>,
    pub firmware_version: Option<u32>,
    pub firmware_verified: bool,
    pub read_only: bool,
    pub read_only_reason: Option<String>,
    pub error: Option<String>,
    pub populated: Vec<bool>,
    pub trip_mm: Vec<f32>,
    pub peak_mm: Vec<f32>,
    pub adc_raw: Vec<u16>,
    pub focus_key: Option<(usize, usize)>,
    pub focus_trace: Vec<f32>,
    pub hid_interfaces: Vec<HidInterfaceInfo>,
    pub config: Option<FullConfig>,
    pub config_seq: u64,
    pub active_profile: Option<String>,
    pub onboard_profile: Option<String>,
    pub foreground_exe: Option<String>,
}

impl Default for Snapshot {
    fn default() -> Self {
        let board = default_board();
        let total = board.total_keys();
        Self {
            connected: false,
            in_bootloader: false,
            board,
            protocol_version: None,
            firmware_version: None,
            firmware_verified: true,
            read_only: false,
            read_only_reason: None,
            error: None,
            populated: vec![true; total],
            trip_mm: vec![0.0; total],
            peak_mm: vec![0.0; total],
            adc_raw: Vec::new(),
            focus_key: None,
            focus_trace: Vec::new(),
            hid_interfaces: Vec::new(),
            config: None,
            config_seq: 0,
            active_profile: None,
            onboard_profile: read_onboard_name(&onboard_path()),
            foreground_exe: None,
        }
    }
}

pub enum DeviceCommand {
    RefreshConfig,
    ResetPeaks,
    SetFocusKey(Option<(usize, usize)>),
    SetPollAdc(bool),
    SetActuation { indices: Vec<usize>, mm: f32 },
    SetRapidTrigger { indices: Vec<usize>, enabled: bool, press_mm: f32, release_mm: f32 },
    SetDeadBand(DeadBand),
    SetFeature(KeyboardFeature),
    SetGameMode(u8),
    SetBottomOptimize(u8),
    SetLight(LightInfo),
    SetBoxLight(BoxLight),
    SetKeyColors { indices: Vec<usize>, color: [u8; 3] },
    SetAllKeyColors(Vec<[u8; 3]>),
    SetKeycode { layer: usize, row: usize, col: usize, keycode: u16 },
    SetKeycodes { layer: usize, positions: Vec<(usize, usize)>, keycode: u16 },
    ApplyKeymap(Vec<u8>),
    ApplyProfileKeymap(String),
    ResetKeymap,
    SetAdvancedKey(AdvancedKey),
    SetDks(Dks),
    SetTapDance(TapDance),
    SetMacroSlot { index: usize, steps: Vec<MacroStep> },
    ApplyProfile { name: String, flash_op: FlashOp },
    CommitToFlash,
    StartCalibration,
    FinishCalibration,
    FactoryReset,
    UnlockBoard(u16),
    LockBoard(u16),
    SetGain([f32; 3]),
    ExportBackup(PathBuf),
    RestoreBackup { path: PathBuf, commit_flash: bool },
    RunDiagnostic { write_probe: bool, tx: std::sync::mpsc::Sender<Result<DiagnosticOutput, String>> },
    ReloadSwitcher,
    Respond(Sender<Result<String, String>>, Box<DeviceCommand>),
}

pub struct SamplerHandle {
    pub snapshot: Arc<RwLock<Snapshot>>,
    tx: Sender<DeviceCommand>,
    stop: Arc<AtomicBool>,
    pub last_toast: Arc<RwLock<Option<(String, bool)>>>,
}

impl SamplerHandle {
    pub fn start() -> Self {
        let snapshot = Arc::new(RwLock::new(Snapshot::default()));
        let last_toast = Arc::new(RwLock::new(None));
        let (tx, rx) = unbounded::<DeviceCommand>();
        let stop = Arc::new(AtomicBool::new(false));

        let fg_app: Arc<RwLock<Option<ForegroundApp>>> = Arc::new(RwLock::new(None));
        let fg_poll_ms = Arc::new(AtomicU64::new(250));

        // Dedicated thread for foreground window queries so X11/D-Bus/IPC never stalls HID polling.
        let fg_clone = Arc::clone(&fg_app);
        let fg_poll_clone = Arc::clone(&fg_poll_ms);
        let fg_stop = Arc::clone(&stop);
        thread::spawn(move || {
            while !fg_stop.load(Ordering::Relaxed) {
                let app = foreground_app();
                if let Ok(mut g) = fg_clone.write() {
                    *g = app;
                }
                let ms = fg_poll_clone.load(Ordering::Relaxed).clamp(100, 2000);
                thread::sleep(Duration::from_millis(ms));
            }
        });

        let snap_clone = Arc::clone(&snapshot);
        let toast_clone = Arc::clone(&last_toast);
        let stop_clone = Arc::clone(&stop);

        thread::spawn(move || {
            worker_loop(
                snap_clone,
                toast_clone,
                rx,
                stop_clone,
                fg_app,
                fg_poll_ms,
            );
        });

        Self {
            snapshot,
            tx,
            stop,
            last_toast,
        }
    }

    pub fn send(&self, cmd: DeviceCommand) {
        let _ = self.tx.send(cmd);
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn read_snapshot(&self) -> Snapshot {
        self.snapshot.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn take_toast(&self) -> Option<(String, bool)> {
        self.last_toast.write().ok().and_then(|mut g| g.take())
    }
}

fn set_toast(toast: &Arc<RwLock<Option<(String, bool)>>>, msg: impl Into<String>, is_err: bool) {
    if let Ok(mut g) = toast.write() {
        *g = Some((msg.into(), is_err));
    }
}

struct WorkerState {
    focus_key: Option<(usize, usize)>,
    focus_trace: Vec<f32>,
    poll_adc: bool,
}

fn worker_loop(
    snapshot: Arc<RwLock<Snapshot>>,
    toast: Arc<RwLock<Option<(String, bool)>>>,
    rx: crossbeam_channel::Receiver<DeviceCommand>,
    stop: Arc<AtomicBool>,
    fg_app: Arc<RwLock<Option<ForegroundApp>>>,
    fg_poll_ms: Arc<AtomicU64>,
) {
    let mut unlocked = UnlockedBoards::load(&unlocked_boards_path());
    let mut settings = AppSettings::load(&settings_path());
    let mut switcher_cfg = SwitcherConfig::load(&switcher_config_path());
    fg_poll_ms.store(switcher_cfg.poll_ms.max(100), Ordering::Relaxed);
    let mut switcher_state = SwitcherState::default();
    let mut last_switch_poll = Instant::now();
    let mut wstate = WorkerState {
        focus_key: None,
        focus_trace: Vec::with_capacity(FOCUS_TRACE_LEN),
        poll_adc: false,
    };

    while !stop.load(Ordering::Relaxed) {
        // Attempt connection
        let mut kb = match Mad68::open(true, false, settings.led_gain, &unlocked) {
            Ok(kb) => kb,
            Err(err) => {
                let ifaces = find_interfaces().unwrap_or_default();
                let fg_exe = fg_app
                    .read()
                    .ok()
                    .and_then(|g| g.as_ref().map(|a| a.exe.clone()));
                if let Ok(mut s) = snapshot.write() {
                    s.connected = false;
                    s.in_bootloader = matches!(err, crate::device::DeviceError::InBootloader);
                    s.error = Some(err.to_string());
                    s.hid_interfaces = ifaces;
                    s.foreground_exe = fg_exe;
                }
                while let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        DeviceCommand::UnlockBoard(pid) => {
                            unlocked.set_unlocked(pid, true);
                            let _ = unlocked.save(&unlocked_boards_path());
                        }
                        DeviceCommand::LockBoard(pid) => {
                            unlocked.set_unlocked(pid, false);
                            let _ = unlocked.save(&unlocked_boards_path());
                        }
                        DeviceCommand::SetGain(g) => {
                            settings.led_gain = g;
                            let _ = settings.save(&settings_path());
                        }
                        DeviceCommand::ReloadSwitcher => {
                            switcher_cfg = SwitcherConfig::load(&switcher_config_path());
                            fg_poll_ms.store(switcher_cfg.poll_ms.max(100), Ordering::Relaxed);
                        }
                        DeviceCommand::SetFocusKey(k) => {
                            if wstate.focus_key != k {
                                wstate.focus_key = k;
                                wstate.focus_trace.clear();
                            }
                        }
                        DeviceCommand::SetPollAdc(p) => wstate.poll_adc = p,
                        DeviceCommand::RunDiagnostic { tx, .. } => {
                            let _ = tx.send(Err("Keyboard is not connected".into()));
                        }
                        _ => {}
                    }
                }
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        // Connected: read initial full config
        let populated = kb.populated_mask();
        let full_cfg = kb.read_full_config().ok();
        let ifaces = find_interfaces().unwrap_or_default();
        {
            if let Ok(mut s) = snapshot.write() {
                s.connected = true;
                s.in_bootloader = false;
                s.board = kb.spec.clone();
                s.protocol_version = kb.protocol_version;
                s.firmware_version = kb.firmware_version;
                s.firmware_verified = kb.protocol_version == Some(KNOWN_PROTOCOL_VERSION);
                s.read_only = !kb.writes;
                s.read_only_reason = kb.read_only_reason.clone();
                s.error = None;
                s.populated = populated.clone();
                let total = kb.total_keys();
                s.trip_mm = vec![0.0; total];
                s.peak_mm = vec![0.0; total];
                s.adc_raw.clear();
                s.hid_interfaces = ifaces;
                s.config = full_cfg;
                s.config_seq = s.config_seq.wrapping_add(1);
            }
        }

        // Connected polling loop
        'poll: while !stop.load(Ordering::Relaxed) {
            // 1. Drain and coalesce commands so rapid slider drags apply immediately without queue lag
            let mut pending = Vec::new();
            while let Ok(cmd) = rx.try_recv() {
                pending.push(cmd);
            }
            let coalesced = coalesce_commands(pending);
            for cmd in coalesced {
                let res = handle_command(
                    &mut kb,
                    cmd,
                    &snapshot,
                    &mut unlocked,
                    &mut settings,
                    &mut switcher_cfg,
                    &fg_poll_ms,
                    &mut wstate,
                );
                if let Err(msg) = res {
                    set_toast(&toast, msg, true);
                } else if let Ok(Some(ok_msg)) = res {
                    set_toast(&toast, ok_msg, false);
                }
            }

            // 2. High-rate burst sampling for focused key oscilloscope trace
            if let Some((fr, fc)) = wstate.focus_key {
                for _ in 0..FOCUS_BURST_SAMPLES {
                    if let Ok(mm) = kb.read_trip_mm(fr, fc) {
                        wstate.focus_trace.push(mm);
                        if wstate.focus_trace.len() > FOCUS_TRACE_LEN {
                            let excess = wstate.focus_trace.len() - FOCUS_TRACE_LEN;
                            wstate.focus_trace.drain(0..excess);
                        }
                    } else {
                        break;
                    }
                }
            }

            // 3. Poll live trip telemetry (~30ms)
            match kb.read_trip_raw() {
                Ok(raw) => {
                    let adc_opt = if wstate.poll_adc {
                        kb.read_adc_raw().ok()
                    } else {
                        None
                    };
                    if let Ok(mut s) = snapshot.write() {
                        if s.trip_mm.len() != raw.len() {
                            s.trip_mm = vec![0.0; raw.len()];
                            s.peak_mm = vec![0.0; raw.len()];
                        }
                        for (i, &r) in raw.iter().enumerate() {
                            let mm = travel_to_mm(r);
                            s.trip_mm[i] = mm;
                            if mm > s.peak_mm[i] {
                                s.peak_mm[i] = mm;
                            }
                        }
                        if let Some(adc) = adc_opt {
                            s.adc_raw = adc;
                        }
                        s.focus_key = wstate.focus_key;
                        s.focus_trace = wstate.focus_trace.clone();
                    }
                }
                Err(_) => {
                    break 'poll; // Disconnected
                }
            }

            // 4. App Switcher evaluation (reads cached foreground window without blocking HID)
            if last_switch_poll.elapsed() >= Duration::from_millis(switcher_cfg.poll_ms.max(100)) {
                last_switch_poll = Instant::now();
                let fg = fg_app.read().ok().and_then(|g| g.clone());
                if let Ok(mut s) = snapshot.write() {
                    s.foreground_exe = fg.as_ref().map(|a| a.exe.clone());
                }
                if switcher_cfg.enabled {
                    if let Some(target) = switcher_state.evaluate(&switcher_cfg, fg.as_ref()) {
                        if let Ok(p) = profile_path(&profile_dir(), &target) {
                            if let Ok(prof) = Profile::load(&p) {
                                if kb.apply_profile(&prof, FlashOp::Normal).is_ok() {
                                    let cfg_opt = kb.read_full_config().ok();
                                    if let Ok(mut s) = snapshot.write() {
                                        s.active_profile = Some(target.clone());
                                        s.config = cfg_opt;
                                        s.config_seq = s.config_seq.wrapping_add(1);
                                    }
                                    set_toast(&toast, format!("Switched to profile: {target}"), false);
                                }
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(if wstate.focus_key.is_some() { 16 } else { 28 }));
        }
    }
}

fn coalesce_commands(cmds: Vec<DeviceCommand>) -> Vec<DeviceCommand> {
    let mut out: Vec<DeviceCommand> = Vec::with_capacity(cmds.len());
    for cmd in cmds {
        match &cmd {
            DeviceCommand::SetLight(_) => {
                out.retain(|c| !matches!(c, DeviceCommand::SetLight(_)));
            }
            DeviceCommand::SetBoxLight(_) => {
                out.retain(|c| !matches!(c, DeviceCommand::SetBoxLight(_)));
            }
            DeviceCommand::SetDeadBand(_) => {
                out.retain(|c| !matches!(c, DeviceCommand::SetDeadBand(_)));
            }
            DeviceCommand::SetGain(_) => {
                out.retain(|c| !matches!(c, DeviceCommand::SetGain(_)));
            }
            DeviceCommand::SetFocusKey(_) => {
                out.retain(|c| !matches!(c, DeviceCommand::SetFocusKey(_)));
            }
            DeviceCommand::SetActuation { indices, .. } => {
                out.retain(|c| !matches!(c, DeviceCommand::SetActuation { indices: idx, .. } if idx == indices));
            }
            DeviceCommand::SetRapidTrigger { indices, .. } => {
                out.retain(|c| !matches!(c, DeviceCommand::SetRapidTrigger { indices: idx, .. } if idx == indices));
            }
            DeviceCommand::SetKeyColors { indices, .. } => {
                out.retain(|c| !matches!(c, DeviceCommand::SetKeyColors { indices: idx, .. } if idx == indices));
            }
            _ => {}
        }
        out.push(cmd);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn handle_command(
    kb: &mut Mad68,
    cmd: DeviceCommand,
    snapshot: &Arc<RwLock<Snapshot>>,
    unlocked: &mut UnlockedBoards,
    settings: &mut AppSettings,
    switcher_cfg: &mut SwitcherConfig,
    fg_poll_ms: &Arc<AtomicU64>,
    wstate: &mut WorkerState,
) -> Result<Option<String>, String> {
    match cmd {
        DeviceCommand::RefreshConfig => {
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(None)
        }
        DeviceCommand::ResetPeaks => {
            wstate.focus_trace.clear();
            if let Ok(mut s) = snapshot.write() {
                s.peak_mm.fill(0.0);
                s.focus_trace.clear();
            }
            Ok(None)
        }
        DeviceCommand::SetFocusKey(k) => {
            if wstate.focus_key != k {
                wstate.focus_key = k;
                wstate.focus_trace.clear();
                if let Ok(mut s) = snapshot.write() {
                    s.focus_key = k;
                    s.focus_trace.clear();
                }
            }
            Ok(None)
        }
        DeviceCommand::SetPollAdc(p) => {
            wstate.poll_adc = p;
            Ok(None)
        }
        DeviceCommand::SetActuation { indices, mm } => {
            let mut vals = kb.read_actuation_bulk().map_err(|e| e.to_string())?;
            for i in indices {
                if i < vals.len() {
                    vals[i] = mm;
                }
            }
            kb.write_actuation_bulk(&vals, 0, FlashOp::Normal).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config {
                    c.actuation = vals;
                }
            }
            Ok(None)
        }
        DeviceCommand::SetRapidTrigger { indices, enabled, press_mm, release_mm } => {
            let mut specs = kb.read_rapid_trigger_bulk().map_err(|e| e.to_string())?;
            for i in indices {
                if i < specs.len() {
                    specs[i].enabled = enabled;
                    specs[i].press_mm = press_mm;
                    specs[i].release_mm = release_mm;
                }
            }
            kb.write_rapid_trigger_bulk(&specs, 0, FlashOp::Normal).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config {
                    c.rapid_trigger = specs;
                }
            }
            Ok(None)
        }
        DeviceCommand::SetDeadBand(db) => {
            kb.write_dead_band(&db).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.dead_band = db; }
            }
            Ok(None)
        }
        DeviceCommand::SetFeature(f) => {
            kb.write_feature(&f).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.feature = f; }
            }
            Ok(Some("Features updated".into()))
        }
        DeviceCommand::SetGameMode(m) => {
            kb.write_game_mode(m).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.game_mode = m; }
            }
            Ok(Some("Game mode updated".into()))
        }
        DeviceCommand::SetBottomOptimize(b) => {
            kb.write_bottom_optimize(b).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.bottom_optimize = b; }
            }
            Ok(Some("Bottom optimize updated".into()))
        }
        DeviceCommand::SetLight(info) => {
            kb.write_light_info(&info).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.light = info; }
            }
            Ok(None)
        }
        DeviceCommand::SetBoxLight(box_l) => {
            kb.write_box_light(&box_l).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.box_light = box_l; }
            }
            Ok(None)
        }
        DeviceCommand::SetKeyColors { indices, color } => {
            if indices.len() <= 4 {
                for &i in &indices {
                    if i < kb.total_keys() {
                        let (r, c) = kb.key_rowcol(i);
                        kb.write_key_colors(r, c, &[color]).map_err(|e| e.to_string())?;
                    }
                }
                if let Ok(mut s) = snapshot.write() {
                    if let Some(ref mut cfg) = s.config {
                        for &i in &indices {
                            if i < cfg.key_colors.len() {
                                cfg.key_colors[i] = color;
                            }
                        }
                    }
                }
            } else {
                let cached = snapshot
                    .read()
                    .ok()
                    .and_then(|s| s.config.as_ref().map(|c| c.key_colors.clone()))
                    .filter(|v| v.len() == kb.total_keys());
                let mut colors = match cached {
                    Some(v) => v,
                    None => kb.read_all_key_colors().map_err(|e| e.to_string())?,
                };
                for i in indices {
                    if i < colors.len() {
                        colors[i] = color;
                    }
                }
                kb.write_all_key_colors(&colors).map_err(|e| e.to_string())?;
                if let Ok(mut s) = snapshot.write() {
                    if let Some(ref mut c) = s.config {
                        c.key_colors = colors;
                    }
                }
            }
            Ok(None)
        }
        DeviceCommand::SetAllKeyColors(colors) => {
            kb.write_all_key_colors(&colors).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.key_colors = colors; }
            }
            Ok(Some("Key colors updated".into()))
        }
        DeviceCommand::SetKeycode { layer, row, col, keycode } => {
            kb.write_keycode(layer, row, col, keycode).map_err(|e| e.to_string())?;
            let km = kb.read_keymap().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.keymap = km; }
            }
            Ok(Some("Keycode updated (EEPROM)".into()))
        }
        DeviceCommand::SetKeycodes { layer, positions, keycode } => {
            for (row, col) in positions {
                kb.write_keycode(layer, row, col, keycode).map_err(|e| e.to_string())?;
            }
            let km = kb.read_keymap().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.keymap = km; }
            }
            Ok(Some("Keycodes updated (EEPROM)".into()))
        }
        DeviceCommand::ApplyKeymap(km) => {
            kb.write_keymap(&km).map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.keymap = km; }
            }
            Ok(Some("Keymap written to keyboard".into()))
        }
        DeviceCommand::ApplyProfileKeymap(name) => {
            let p = profile_path(&profile_dir(), &name)?;
            let prof = Profile::load(&p)?;
            let want = prof
                .to_keymap_bytes(kb.spec.layers, kb.spec.rows, kb.spec.cols)
                .ok_or_else(|| "Profile has no compatible keymap for this board".to_string())?;
            kb.write_keymap(&want).map_err(|e| e.to_string())?;
            let km = kb.read_keymap().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.keymap = km; }
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some(format!("Keymap from '{name}' written to EEPROM")))
        }
        DeviceCommand::ResetKeymap => {
            kb.dangerous = true;
            let res = kb.reset_keymap();
            kb.dangerous = false;
            res.map_err(|e| e.to_string())?;
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some("Keymap reset to factory defaults".into()))
        }
        DeviceCommand::SetAdvancedKey(ak) => {
            kb.write_advanced_key(&ak).map_err(|e| e.to_string())?;
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some("Advanced key updated".into()))
        }
        DeviceCommand::SetDks(dks) => {
            kb.write_dks(&dks).map_err(|e| e.to_string())?;
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some("DKS updated".into()))
        }
        DeviceCommand::SetTapDance(td) => {
            kb.write_tap_dance(&td).map_err(|e| e.to_string())?;
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some("Hold/Click (MT) updated".into()))
        }
        DeviceCommand::SetMacroSlot { index, steps } => {
            kb.write_macro_slot(index, &steps).map_err(|e| e.to_string())?;
            let macros = kb.read_macro_steps().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                if let Some(ref mut c) = s.config { c.macros = macros; }
            }
            Ok(Some(format!("Macro M{index} saved")))
        }
        DeviceCommand::ApplyProfile { name, flash_op } => {
            let p = profile_path(&profile_dir(), &name)?;
            let prof = Profile::load(&p)?;
            kb.apply_profile(&prof, flash_op).map_err(|e| e.to_string())?;
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if flash_op == FlashOp::EraseAndWrite {
                let _ = write_onboard_name(&onboard_path(), &name);
            }
            if let Ok(mut s) = snapshot.write() {
                s.active_profile = Some(name.clone());
                if flash_op == FlashOp::EraseAndWrite {
                    s.onboard_profile = Some(name.clone());
                }
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some(if flash_op == FlashOp::EraseAndWrite {
                format!("Profile '{name}' saved to keyboard flash")
            } else {
                format!("Applied profile '{name}'")
            }))
        }
        DeviceCommand::CommitToFlash => {
            kb.commit_to_flash().map_err(|e| e.to_string())?;
            Ok(Some("Current settings committed to flash".into()))
        }
        DeviceCommand::StartCalibration => {
            kb.dangerous = true;
            let res = kb.start_calibration();
            kb.dangerous = false;
            res.map_err(|e| e.to_string())?;
            Ok(Some("Calibration started — press every key fully".into()))
        }
        DeviceCommand::FinishCalibration => {
            kb.dangerous = true;
            let res = kb.finish_calibration();
            kb.dangerous = false;
            res.map_err(|e| e.to_string())?;
            kb.clear_populated_cache();
            let populated = kb.populated_mask();
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                s.populated = populated;
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some("Calibration finished".into()))
        }
        DeviceCommand::FactoryReset => {
            kb.dangerous = true;
            let res = kb.factory_reset();
            kb.dangerous = false;
            res.map_err(|e| e.to_string())?;
            kb.clear_populated_cache();
            let populated = kb.populated_mask();
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                s.populated = populated;
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some("Keyboard factory reset complete".into()))
        }
        DeviceCommand::UnlockBoard(pid) => {
            unlocked.set_unlocked(pid, true);
            let _ = unlocked.save(&unlocked_boards_path());
            kb.writes = true;
            kb.read_only_reason = None;
            if let Ok(mut s) = snapshot.write() {
                s.read_only = false;
                s.read_only_reason = None;
            }
            Ok(Some("Board unlocked for writing".into()))
        }
        DeviceCommand::LockBoard(pid) => {
            unlocked.set_unlocked(pid, false);
            let _ = unlocked.save(&unlocked_boards_path());
            if !kb.spec.verified {
                kb.writes = false;
                let reason = format!(
                    "{} ({:#06x}) is unverified; opened read-only",
                    kb.spec.name, kb.spec.product_id
                );
                kb.read_only_reason = Some(reason.clone());
                if let Ok(mut s) = snapshot.write() {
                    s.read_only = true;
                    s.read_only_reason = Some(reason);
                }
            }
            Ok(Some("Board locked (read-only)".into()))
        }
        DeviceCommand::SetGain(g) => {
            settings.led_gain = g;
            let _ = settings.save(&settings_path());
            kb.set_gain(g);
            Ok(None)
        }
        DeviceCommand::ExportBackup(path) => {
            export_backup(kb, &path).map_err(|e| e.to_string())?;
            Ok(Some(format!("Backup saved to {}", path.display())))
        }
        DeviceCommand::RestoreBackup { path, commit_flash } => {
            restore_backup(kb, &path, commit_flash).map_err(|e| e.to_string())?;
            let cfg = kb.read_full_config().map_err(|e| e.to_string())?;
            if let Ok(mut s) = snapshot.write() {
                s.config = Some(cfg);
                s.config_seq = s.config_seq.wrapping_add(1);
            }
            Ok(Some("Backup restored".into()))
        }
        DeviceCommand::RunDiagnostic { write_probe, tx } => {
            let res = save_and_prepare_diagnostic(kb, write_probe);
            let _ = tx.send(res);
            Ok(None)
        }
        DeviceCommand::ReloadSwitcher => {
            *switcher_cfg = SwitcherConfig::load(&switcher_config_path());
            fg_poll_ms.store(switcher_cfg.poll_ms.max(100), Ordering::Relaxed);
            Ok(None)
        }
        DeviceCommand::Respond(tx, inner) => {
            let r = handle_command(
                kb,
                *inner,
                snapshot,
                unlocked,
                settings,
                switcher_cfg,
                fg_poll_ms,
                wstate,
            );
            let msg = match &r {
                Ok(Some(m)) => Ok(m.clone()),
                Ok(None) => Ok("Done".into()),
                Err(e) => Err(e.clone()),
            };
            let _ = tx.send(msg);
            r
        }
    }
}
