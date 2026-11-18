//! Main OpenMAD-Rust iced application state, command dispatch, async tasks, and subscriptions.

use crate::i18n::Lang;
use crate::theme::dynamic_accent;
use crate::tray::{TrayAction, TrayHandle, TrayMenuState};
use crate::widgets::color_picker::{hsv_to_rgb, rgb_to_hsv};
use crate::widgets::keyboard::KeyboardEvent;
use iced::keyboard::{key, Key, Location};
use iced::{time, window, Color, Element, Subscription, Task};
use openmad_core::device::FullConfig;
use openmad_core::diagnostics::DiagnosticOutput;
use openmad_core::features::{
    AdvancedKey, BoxLight, DeadBand, Dks, KeyboardFeature, LightInfo, MacroStep, TapDance,
    LIGHT_EFFECTS,
};
use openmad_core::keycodes;
use openmad_core::paths::{onboard_path, profile_dir, settings_path, switcher_config_path};
use openmad_core::profile::{
    key_str, list_profiles, profile_path, write_onboard_name, Profile, RapidTriggerSpec,
    TriggerBinding,
};
use openmad_core::protocol::{AdvancedKeyMode, FlashOp, BRIGHTNESS_MAX};
use openmad_core::sampler::{DeviceCommand, SamplerHandle, Snapshot};
use openmad_core::settings::{set_start_at_login, AppSettings};
use openmad_core::switcher::{
    list_windowed_processes, FallbackMode, Rule, SwitcherConfig, WindowedProcess,
};
use openmad_core::updater::{
    check_for_update, download_and_apply_update, UpdateInfo, REPO_SLUG,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const COLOR_PRESETS: &[(u8, u8, u8)] = &[
    (255, 255, 255),
    (255, 59, 48),
    (255, 149, 0),
    (255, 214, 10),
    (48, 209, 88),
    (100, 210, 255),
    (10, 132, 255),
    (191, 90, 242),
    (255, 55, 95),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Keys,
    Lighting,
    Performance,
    Advanced,
    Macros,
    AppTrigger,
    System,
    Settings,
}

impl NavSection {
    pub fn title(self, lang: Lang) -> &'static str {
        match self {
            Self::Keys => lang.t("Remapear Teclas", "Keymap"),
            Self::Lighting => lang.t("Iluminação", "Lighting"),
            Self::Performance => lang.t("Performance", "Performance"),
            Self::Advanced => lang.t("Teclas Avançadas", "Advanced Key"),
            Self::Macros => "Macros",
            Self::AppTrigger => lang.t("Troca por App", "App Trigger"),
            Self::System => lang.t("Sistema & Flash", "System & Flash"),
            Self::Settings => lang.t("Configurações", "Settings"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfSubTab {
    Travel,
    Calibration,
    Performance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPickerTab {
    Keyboard,
    System,
    Macro,
    Layer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvSubTab {
    RappySnappy,
    Socd,
    HoldClick,
    Dks,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvPickTarget {
    Key1,
    Key2,
    MtTap,
    MtHold,
    Dks(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyZone {
    Wasd,
    Arrows,
    Numbers,
    Modifiers,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteProfile(String),
    CommitToFlash,
    SaveProfileOnboard(String),
    StartCalibration,
    ResetKeymap,
    UnlockBoard(u16),
}

#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub title: String,
    pub body: String,
    pub confirm_label: String,
    pub danger: bool,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    WindowCloseRequested(window::Id),
    WindowEvent(window::Id, window::Event),
    CheckMinimized(window::Id, Option<bool>),
    HideToTray,
    DismissToast,
    ToggleLanguage,
    SelectLanguage(String),
    SelectSection(NavSection),
    ToggleKeyboardCollapsed,
    ToggleProfileMenu,
    ToggleSaveMenu,
    CloseMenus,
    ToggleLightingRgb,
    KeycodeSearchChanged(String),
    ApplyPerfPreset {
        actuation: f32,
        rt: bool,
        press: f32,
        release: f32,
    },
    SelectPerfSubTab(PerfSubTab),
    SelectKeyPickerTab(KeyPickerTab),
    SelectAdvSubTab(AdvSubTab),
    SelectLayer(usize),
    Keyboard(KeyboardEvent),
    SelectKeyZone(KeyZone),
    SelectAllKeys,
    SelectNoKeys,
    InvertKeys,
    OpenFirmwareUrl,

    // Confirm modal
    ConfirmDialogAccept,
    ConfirmDialogCancel,

    // Profile management
    SelectProfile(String),
    NewProfileNameChanged(String),
    CreateProfile(bool),
    SaveCurrentProfile,
    RenameProfile,
    DuplicateProfile,
    RequestDeleteProfile,
    ImportProfile,
    ImportProfilePicked(Option<PathBuf>),
    ExportProfile,
    ExportProfilePicked(Option<PathBuf>),
    RequestSaveProfileOnboard,
    SetDefaultProfile(String),

    // Keymap
    KeycodePicked(u16),
    CustomHexInputChanged(String),
    AssignCustomHexKeycode,
    CaptureProfileKeymap,
    ApplyProfileKeymap,

    // Lighting
    LightEffectSelected(u8),
    LightColorPreset(u8, u8, u8),
    LightHsvChanged(f32, f32, f32),
    LightHexChanged(String),
    LightBrightnessChanged(u8),
    LightSpeedChanged(u8),
    LightRChanged(u8),
    LightGChanged(u8),
    LightBChanged(u8),
    PaintSelectedKeys,
    ClearKeyColorsSelection,
    BoxModeChanged(u8),
    BoxColorfulChanged(u8),
    BoxBrightnessChanged(u8),
    BoxSpeedChanged(u8),

    // Performance
    ActuationChanged(f32),
    ApplyActuation,
    RtEnabledToggled(bool),
    RtPressChanged(f32),
    RtReleaseChanged(f32),
    ApplyRapidTrigger,
    DeadBandTopChanged(f32),
    DeadBandBottomChanged(f32),
    ResetPeaks,
    RequestStartCalibration,
    FinishCalibration,
    GameModeToggled(bool),
    BottomOptimizeToggled(bool),
    FeatureRgbAreaToggled(bool),
    FeatureWasdToggled(bool),
    FeatureMacToggled(bool),
    FeatureWinLockToggled(bool),
    FeatureNkroToggled(bool),

    // Advanced Key
    AdvSlotSelected(usize),
    AdvPickTargetSelected(Option<AdvPickTarget>),
    AdvModeChanged(AdvancedKeyMode),
    AdvApcChanged(f32),
    AdvRtToggled(bool),
    SaveAdvancedKey,
    ClearAdvancedKey,
    MtTimerChanged(String),
    SaveTapDance,
    ClearTapDance,
    DksToggleBit(usize, usize),
    SaveDks,
    ClearDks,

    // App Trigger
    ToggleAutoSwitcher(bool),
    SetFallbackMode(FallbackMode),
    TriggerExeChanged(String),
    TriggerTitleChanged(String),
    BrowseTriggerExe,
    TriggerExePicked(Option<PathBuf>),
    AddTriggerRule,
    RemoveTriggerRule(usize),
    MoveTriggerRuleUp(usize),
    MoveTriggerRuleDown(usize),
    SwitcherDebounceChanged(String),
    SwitcherDwellChanged(String),
    RefreshProcesses,
    ProcessesRefreshed(Vec<WindowedProcess>),
    PickRunningProcess(String),

    // System & Diagnostics
    RequestUnlockBoard,
    LockBoard,
    RequestCommitToFlash,
    ExportBackup,
    ExportBackupPicked(Option<PathBuf>),
    RestoreBackup,
    RestoreBackupPicked(Option<PathBuf>),
    FactoryResetInputChanged(String),
    ConfirmFactoryReset,
    RequestResetKeymap,
    DiagWriteProbeToggled(bool),
    RunDiagnostic,
    DiagOutputReady(Box<Result<DiagnosticOutput, String>>),
    CloseDiagModal,
    CopyDiagJson,
    OpenDiagIssueUrl,

    // Macros
    SelectMacroSlot(usize),
    MacroStepKindChanged(String),
    MacroStepValChanged(String),
    ToggleMacroPicker,
    ToggleMacroRecording,
    MacroKeyboardEvent(iced::keyboard::Event),
    AddMacroStep,
    MoveMacroStepUp(usize),
    MoveMacroStepDown(usize),
    RemoveMacroStep(usize),
    SaveMacroSlot,
    ClearMacroSlot,

    // Settings
    CheckUpdates,
    UpdateChecked(Result<UpdateInfo, String>),
    InstallUpdate,
    UpdateApplied(Result<String, String>),
    GainRChanged(f32),
    GainGChanged(f32),
    GainBChanged(f32),
    SetGainPreset([f32; 3]),
    PreviewLedColor([u8; 3]),
    RestoreLedPreview,
    StartAtLoginToggled(bool),
    CloseToTrayToggled(bool),
    CheckUpdatesPrefToggled(bool),

    // Headless screenshot utility mode
    ScreenshotCaptured(String, window::Screenshot),
}

pub struct OpenMadApp {
    pub(crate) sampler: SamplerHandle,
    pub(crate) tray: Option<TrayHandle>,
    pub(crate) snap: Snapshot,
    pub(crate) window_id: Option<window::Id>,

    pub(crate) lang: Lang,
    pub(crate) section: NavSection,
    pub(crate) perf_sub: PerfSubTab,
    pub(crate) pick_tab: KeyPickerTab,
    pub(crate) adv_sub: AdvSubTab,
    pub(crate) layer: usize,
    pub(crate) selected_keys: BTreeSet<usize>,
    pub(crate) keyboard_collapsed: bool,
    pub(crate) profile_menu_open: bool,
    pub(crate) save_menu_open: bool,
    pub(crate) lighting_advanced_rgb: bool,
    pub(crate) keycode_search: String,
    pub(crate) custom_hex_input: String,

    pub(crate) profiles: Vec<String>,
    pub(crate) selected_profile: Option<String>,
    pub(crate) new_profile_name: String,

    pub(crate) light_draft: LightInfo,
    pub(crate) light_hue: f32,
    pub(crate) light_sat: f32,
    pub(crate) light_val: f32,
    pub(crate) light_hex: String,
    pub(crate) box_draft: BoxLight,

    pub(crate) actuation_mm: f32,
    pub(crate) rt_enabled: bool,
    pub(crate) rt_press_mm: f32,
    pub(crate) rt_release_mm: f32,
    pub(crate) dead_top_mm: f32,
    pub(crate) dead_bot_mm: f32,

    pub(crate) adv_slot: usize,
    pub(crate) adv_mode: AdvancedKeyMode,
    pub(crate) adv_pick_target: Option<AdvPickTarget>,
    pub(crate) adv_k1_r: String,
    pub(crate) adv_k1_c: String,
    pub(crate) adv_k2_r: String,
    pub(crate) adv_k2_c: String,
    pub(crate) adv_apc: f32,
    pub(crate) adv_rt: bool,
    pub(crate) mt_tap: String,
    pub(crate) mt_hold: String,
    pub(crate) mt_timer: String,
    pub(crate) dks_draft: Dks,

    pub(crate) switcher_cfg: SwitcherConfig,
    pub(crate) trigger_exe: String,
    pub(crate) trigger_title: String,
    pub(crate) processes: Vec<WindowedProcess>,

    pub(crate) macro_slot: usize,
    pub(crate) macro_steps: Vec<MacroStep>,
    pub(crate) macro_step_kind: String,
    pub(crate) macro_step_val: String,
    pub(crate) macro_picker_open: bool,
    pub(crate) macro_recording: bool,
    pub(crate) macro_rec_last: Option<Instant>,

    pub(crate) factory_confirm: String,
    pub(crate) confirm_dialog: Option<ConfirmDialog>,
    pub(crate) diag_write_probe: bool,
    pub(crate) diag_running: bool,
    pub(crate) diag_output: Option<DiagnosticOutput>,

    pub(crate) settings: AppSettings,
    pub(crate) update_info: Option<Result<UpdateInfo, String>>,
    pub(crate) update_installing: bool,
    pub(crate) led_preview_restore: Option<LightInfo>,
    pub(crate) toast: Option<(String, bool, u8)>,
    pub(crate) last_config_seq: u64,
    pub(crate) ram_dirty: bool,
    pub(crate) screenshot_target: Option<String>,
    pub(crate) screenshot_ticks: u32,
}

impl OpenMadApp {
    pub fn new() -> (Self, Task<Message>) {
        let sampler = SamplerHandle::start();
        let profiles = list_profiles(&profile_dir());
        let settings = AppSettings::load(&settings_path());
        let lang = Lang::from_code(&settings.language);
        let switcher_cfg = SwitcherConfig::load(&switcher_config_path());
        let selected_profile = std::env::var("OPENMAD_ACTIVE_PROFILE")
            .ok()
            .filter(|s| !s.is_empty() && profiles.contains(s))
            .or_else(|| profiles.first().cloned());
        let tray = TrayHandle::new(TrayMenuState {
            profiles: profiles.clone(),
            active_profile: selected_profile.clone(),
            switcher_enabled: switcher_cfg.enabled,
            is_pt: lang.is_pt(),
        });
        let snap = sampler.read_snapshot();
        let processes = Vec::new();

        let start_minimized = std::env::args().any(|a| a == "--minimized" || a == "--tray");
        let screenshot_target = std::env::args()
            .find_map(|a| a.strip_prefix("--screenshot=").map(|s| s.to_string()));
        let (window_id, init_task) = if start_minimized && screenshot_target.is_none() {
            sampler.set_ui_open(false);
            sampler.set_telemetry_active(false);
            (None, Task::none())
        } else {
            let (win_id, open_task) = window::open(crate::window_settings());
            sampler.set_ui_open(true);
            sampler.set_telemetry_active(true);
            (Some(win_id), open_task.discard())
        };

        let update_task = if settings.check_updates {
            Task::perform(
                async {
                    tokio_blocking(check_for_update).await
                },
                Message::UpdateChecked,
            )
        } else {
            Task::none()
        };

        let app = Self {
            sampler,
            tray,
            snap,
            window_id,
            lang,
            section: NavSection::Performance,
            perf_sub: PerfSubTab::Travel,
            pick_tab: KeyPickerTab::Keyboard,
            adv_sub: AdvSubTab::RappySnappy,
            layer: 0,
            selected_keys: BTreeSet::new(),
            keyboard_collapsed: false,
            profile_menu_open: false,
            save_menu_open: false,
            lighting_advanced_rgb: false,
            keycode_search: String::new(),
            custom_hex_input: String::new(),
            profiles,
            selected_profile,
            new_profile_name: String::new(),
            light_draft: LightInfo {
                effect: 13,
                speed: 128,
                r: 10,
                g: 132,
                b: 255,
                brightness: 160,
            },
            light_hue: 211.0,
            light_sat: 0.96,
            light_val: 1.0,
            light_hex: "#0A84FF".into(),
            box_draft: BoxLight::default(),
            actuation_mm: 2.0,
            rt_enabled: false,
            rt_press_mm: 0.30,
            rt_release_mm: 0.30,
            dead_top_mm: 0.10,
            dead_bot_mm: 0.10,
            adv_slot: 0,
            adv_mode: AdvancedKeyMode::Socd,
            adv_pick_target: None,
            adv_k1_r: "2".into(),
            adv_k1_c: "1".into(),
            adv_k2_r: "2".into(),
            adv_k2_c: "3".into(),
            adv_apc: 0.30,
            adv_rt: true,
            mt_tap: "0x0004".into(),
            mt_hold: "0x00E1".into(),
            mt_timer: "200".into(),
            dks_draft: Dks::empty(0),
            switcher_cfg,
            trigger_exe: String::new(),
            trigger_title: String::new(),
            processes,
            macro_slot: 0,
            macro_steps: Vec::new(),
            macro_step_kind: "tap".into(),
            macro_step_val: "0x0004".into(),
            macro_picker_open: false,
            macro_recording: false,
            macro_rec_last: None,
            factory_confirm: String::new(),
            confirm_dialog: None,
            diag_write_probe: true,
            diag_running: false,
            diag_output: None,
            settings,
            update_info: None,
            update_installing: false,
            led_preview_restore: None,
            toast: None,
            last_config_seq: 0,
            ram_dirty: false,
            screenshot_target,
            screenshot_ticks: 0,
        };
        (app, Task::batch([init_task, update_task]))
    }

    pub fn title(&self, _window: window::Id) -> String {
        if self.snap.connected {
            format!("OpenMAD-Rust — {}", self.snap.board.name)
        } else {
            "OpenMAD-Rust".to_string()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let fast_tick = self.window_id.is_some()
            && (self.section == NavSection::Performance
                || self.toast.is_some()
                || self.macro_recording
                || self.diag_running
                || self.screenshot_target.is_some());
        let tick_ms = if fast_tick { 33 } else { 200 };
        let mut subs = vec![
            time::every(Duration::from_millis(tick_ms)).map(|_| Message::Tick),
            window::close_requests().map(Message::WindowCloseRequested),
            window::events().map(|(id, ev)| Message::WindowEvent(id, ev)),
        ];
        if self.macro_recording {
            subs.push(iced::event::listen_with(|ev, _status, _id| match ev {
                iced::Event::Keyboard(kev) => Some(Message::MacroKeyboardEvent(kev)),
                _ => None,
            }));
        }
        Subscription::batch(subs)
    }

    fn show_window_task(&mut self) -> Task<Message> {
        if let Some(id) = self.window_id {
            self.sync_focus_polling();
            Task::batch([window::minimize(id, false), window::gain_focus(id)])
        } else {
            let (id, open_task) = window::open(crate::window_settings());
            self.window_id = Some(id);
            self.sync_focus_polling();
            open_task.discard()
        }
    }

    fn hide_window_task(&mut self) -> Task<Message> {
        let prev_id = self.window_id.take();
        self.sampler.set_ui_open(false);
        self.sampler.set_telemetry_active(false);
        self.sampler.set_fg_watch_ui(false);
        if self.tray.is_some() && self.screenshot_target.is_none() {
            crate::request_relaunch_minimized(self.selected_profile.as_deref());
            self.tray.take();
            self.sampler.stop_and_join();
            return iced::exit();
        }
        if let Some(id) = prev_id {
            return window::close(id);
        }
        Task::none()
    }

    fn sync_tray_state(&self) {
        if let Some(ref tray) = self.tray {
            tray.update_state(TrayMenuState {
                profiles: self.profiles.clone(),
                active_profile: self.selected_profile.clone(),
                switcher_enabled: self.switcher_cfg.enabled,
                is_pt: self.lang.is_pt(),
            });
        }
    }

    fn sync_focus_polling(&self) {
        let open = self.window_id.is_some();
        self.sampler.set_ui_open(open);
        self.sampler
            .set_telemetry_active(open && self.section == NavSection::Performance);
        self.sampler
            .set_fg_watch_ui(open && self.section == NavSection::AppTrigger);

        if open && self.section == NavSection::Performance && self.perf_sub == PerfSubTab::Travel {
            let cols = self.snap.board.cols.max(1);
            let idx = self
                .selected_keys
                .iter()
                .next()
                .copied()
                .unwrap_or_else(|| (2 * cols + 1).min(self.snap.board.total_keys().saturating_sub(1)));
            self.sampler
                .send(DeviceCommand::SetFocusKey(Some((idx / cols, idx % cols))));
        } else {
            self.sampler.send(DeviceCommand::SetFocusKey(None));
        }
        let poll_adc = open
            && self.section == NavSection::Performance
            && self.perf_sub == PerfSubTab::Calibration;
        self.sampler.send(DeviceCommand::SetPollAdc(poll_adc));
    }

    fn snapshot_into_profile(&self, prof: &mut Profile) {
        prof.light = Some(self.light_draft);
        prof.box_light = Some(self.box_draft);
        prof.dead_band = Some(DeadBand {
            top_mm: self.dead_top_mm,
            bottom_mm: self.dead_bot_mm,
        });

        if let Some(ref cfg) = self.snap.config {
            prof.feature = Some(cfg.feature);
            prof.game_mode = Some(cfg.game_mode);
            prof.bottom_optimize = Some(cfg.bottom_optimize);
            prof.key_colors = Some(cfg.key_colors.clone());
            prof.dks = Some(cfg.dks.clone());
            prof.tap_dance = Some(cfg.tap_dance.clone());
            prof.advanced_keys = Some(cfg.advanced_keys.clone());

            let cols = self.snap.board.cols.max(1);
            if let Some(&def_act) = cfg.actuation.first() {
                prof.actuation_default = Some(def_act);
                prof.actuation_overrides.clear();
                for (i, &val) in cfg.actuation.iter().enumerate() {
                    if (val - def_act).abs() > 0.005 {
                        prof.actuation_overrides.insert(key_str(i / cols, i % cols), val);
                    }
                }
            }
            if let Some(def_rt) = cfg.rapid_trigger.first() {
                let def_spec = RapidTriggerSpec::from(def_rt);
                prof.rapid_trigger_default = Some(def_spec);
                prof.rapid_trigger_overrides.clear();
                for (i, rt) in cfg.rapid_trigger.iter().enumerate() {
                    if rt.enabled != def_spec.enabled
                        || (rt.press_mm - def_spec.press_mm).abs() > 0.005
                        || (rt.release_mm - def_spec.release_mm).abs() > 0.005
                    {
                        prof.rapid_trigger_overrides.insert(
                            key_str(i / cols, i % cols),
                            RapidTriggerSpec::from(rt),
                        );
                    }
                }
            }
        } else {
            prof.actuation_default = Some(self.actuation_mm);
            prof.rapid_trigger_default = Some(RapidTriggerSpec {
                enabled: self.rt_enabled,
                press_mm: self.rt_press_mm,
                release_mm: self.rt_release_mm,
            });
        }
    }

    fn save_current_profile(&mut self) {
        let Some(name) = self.selected_profile.clone() else {
            self.show_toast(
                self.lang.t(
                    "Selecione ou crie um perfil na barra lateral primeiro",
                    "Select or create a profile in the sidebar first",
                ),
                true,
            );
            return;
        };

        let Ok(path) = profile_path(&profile_dir(), &name) else {
            return;
        };
        let mut prof = Profile::load(&path).unwrap_or_else(|_| Profile::blank(&name));
        prof.name = name.clone();
        self.snapshot_into_profile(&mut prof);

        match prof.save(&path) {
            Ok(()) => {
                self.ram_dirty = false;
                self.show_toast(
                    format!(
                        "{} '{name}'",
                        self.lang.t("Perfil salvo:", "Saved profile")
                    ),
                    false,
                );
            }
            Err(e) => self.show_toast(e, true),
        }
    }

    fn sync_profile_triggers_from_switcher(&self) {
        let dir = profile_dir();
        for name in &self.profiles {
            if let Ok(path) = profile_path(&dir, name) {
                if let Ok(mut prof) = Profile::load(&path) {
                    let matching: Vec<TriggerBinding> = self
                        .switcher_cfg
                        .rules
                        .iter()
                        .filter(|r| &r.profile == name)
                        .map(|r| TriggerBinding {
                            exe: r.exe.clone(),
                            title: r.title.clone(),
                        })
                        .collect();
                    if prof.triggers != matching {
                        prof.triggers = matching;
                        prof.trigger_exe = None;
                        prof.trigger_title = None;
                        let _ = prof.save(&path);
                    }
                }
            }
        }
    }

    fn show_toast(&mut self, msg: impl Into<String>, is_err: bool) {
        self.toast = Some((msg.into(), is_err, 110));
    }

    fn target_indices(&self) -> Vec<usize> {
        if self.selected_keys.is_empty() {
            self.snap
                .populated
                .iter()
                .enumerate()
                .filter(|(_, &p)| p)
                .map(|(i, _)| i)
                .collect()
        } else {
            self.selected_keys.iter().copied().collect()
        }
    }

    fn sync_adv_slot_from_config(&mut self, cfg: &FullConfig) {
        if let Some(ak) = cfg.advanced_keys.get(self.adv_slot) {
            self.adv_k1_r = ak.key1_row.to_string();
            self.adv_k1_c = ak.key1_col.to_string();
            self.adv_k2_r = ak.key2_row.to_string();
            self.adv_k2_c = ak.key2_col.to_string();
            if ak.rs_apc_lv > 0 {
                self.adv_apc = (ak.rs_apc_lv as f32) * 0.01;
            }
            self.adv_rt = ak.rt_sw != 0;
            if matches!(
                ak.mode,
                AdvancedKeyMode::Socd
                    | AdvancedKeyMode::SocdKey1
                    | AdvancedKeyMode::SocdKey2
                    | AdvancedKeyMode::SocdBalance
            ) {
                self.adv_mode = ak.mode;
            }
        }
        if let Some(td) = cfg.tap_dance.get(self.adv_slot) {
            self.mt_tap = format!("{:#06x}", td.tap);
            self.mt_hold = format!("{:#06x}", td.hold);
            self.mt_timer = td.timer_ms.to_string();
        }
        if let Some(dks) = cfg.dks.get(self.adv_slot) {
            self.dks_draft = dks.clone();
        }
    }

    fn sync_drafts_from_config(&mut self, cfg: &FullConfig) {
        self.light_draft = cfg.light;
        if self.light_draft.speed == 0 {
            self.light_draft.speed = 128;
        }
        let (h, s, v) = rgb_to_hsv(self.light_draft.r, self.light_draft.g, self.light_draft.b);
        self.light_hue = h;
        self.light_sat = s;
        self.light_val = v;
        self.light_hex = format!(
            "#{:02X}{:02X}{:02X}",
            self.light_draft.r, self.light_draft.g, self.light_draft.b
        );
        self.box_draft = cfg.box_light;
        self.dead_top_mm = cfg.dead_band.top_mm;
        self.dead_bot_mm = cfg.dead_band.bottom_mm;
        let idx = self.selected_keys.iter().next().copied().unwrap_or(0);
        if let Some(&act) = cfg.actuation.get(idx).or_else(|| cfg.actuation.first()) {
            self.actuation_mm = act;
        }
        if let Some(rt) = cfg
            .rapid_trigger
            .get(idx)
            .or_else(|| cfg.rapid_trigger.first())
        {
            self.rt_enabled = rt.enabled;
            self.rt_press_mm = rt.press_mm;
            self.rt_release_mm = rt.release_mm;
        }
        if let Some(m) = cfg.macros.get(self.macro_slot) {
            self.macro_steps = m.clone();
        }
        self.sync_adv_slot_from_config(cfg);
    }

    fn save_language(&mut self, lang: Lang) {
        self.lang = lang;
        self.settings.language = lang.code().to_string();
        let _ = self.settings.save(&settings_path());
        self.sync_tray_state();
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Tick => {
                self.snap = self.sampler.read_snapshot();
                if self.snap.config_seq != self.last_config_seq {
                    if let Some(ref active) = self.snap.active_profile {
                        if self.selected_profile.as_deref() != Some(active.as_str()) {
                            self.selected_profile = Some(active.clone());
                            self.sync_tray_state();
                        }
                    }
                    if let Some(cfg) = self.snap.config.clone() {
                        self.sync_drafts_from_config(&cfg);
                    }
                    self.last_config_seq = self.snap.config_seq;
                    self.ram_dirty = false;
                }
                if let Some((m, err)) = self.sampler.take_toast() {
                    self.show_toast(m, err);
                }
                if let Some((_, _, ref mut ttl)) = self.toast {
                    if *ttl == 0 {
                        self.toast = None;
                    } else {
                        *ttl -= 1;
                    }
                }
                if let Some(ref tray) = self.tray {
                    if let Some(action) = tray.poll_event() {
                        match action {
                            TrayAction::ShowWindow => return self.show_window_task(),
                            TrayAction::HideWindow => return self.hide_window_task(),
                            TrayAction::ToggleSwitcher => {
                                self.switcher_cfg.enabled = !self.switcher_cfg.enabled;
                                let _ = self.switcher_cfg.save(&switcher_config_path());
                                self.sampler.send(DeviceCommand::ReloadSwitcher);
                                self.sync_tray_state();
                                self.show_toast(
                                    if self.switcher_cfg.enabled {
                                        self.lang.t("Troca automática ativada", "Auto-switcher enabled")
                                    } else {
                                        self.lang.t("Troca automática desativada", "Auto-switcher paused")
                                    },
                                    false,
                                );
                            }
                            TrayAction::CommitToFlash => {
                                self.sampler.send(DeviceCommand::CommitToFlash);
                                self.ram_dirty = false;
                            }
                            TrayAction::ApplyProfile(name) => {
                                self.selected_profile = Some(name.clone());
                                self.sync_tray_state();
                                self.sampler.send(DeviceCommand::ApplyProfile {
                                    name,
                                    flash_op: FlashOp::Normal,
                                });
                            }
                            TrayAction::Quit => {
                                self.sampler.stop();
                                return iced::exit();
                            }
                        }
                    }
                }
                if let Some(target) = self.screenshot_target.clone() {
                    self.screenshot_ticks += 1;
                    self.toast = None;
                    self.lang = Lang::Pt;
                    match target.as_str() {
                        "lighting" => self.section = NavSection::Lighting,
                        "performance" => {
                            self.section = NavSection::Performance;
                            self.perf_sub = PerfSubTab::Travel;
                        }
                        "advanced" => self.section = NavSection::Advanced,
                        "macros" => self.section = NavSection::Macros,
                        "app-trigger" => self.section = NavSection::AppTrigger,
                        "remap" => self.section = NavSection::Keys,
                        "system" => self.section = NavSection::System,
                        "settings" => self.section = NavSection::Settings,
                        _ => {}
                    }
                    if self.screenshot_ticks == 36 {
                        if let Some(id) = self.window_id {
                            let out = format!("assets/screenshots/{target}.png");
                            return window::screenshot(id)
                                .map(move |s| Message::ScreenshotCaptured(out.clone(), s));
                        }
                    } else if self.screenshot_ticks > 60 {
                        self.sampler.stop();
                        return iced::exit();
                    }
                }
            }
            Message::ScreenshotCaptured(path, shot) => {
                if let Some(img) = image::RgbaImage::from_raw(
                    shot.size.width,
                    shot.size.height,
                    shot.bytes.to_vec(),
                ) {
                    if let Some(parent) = std::path::Path::new(&path).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = img.save(&path);
                }
                self.sampler.stop();
                return iced::exit();
            }
            Message::WindowCloseRequested(_id) => {
                if self.settings.close_to_tray {
                    return self.hide_window_task();
                } else {
                    self.tray.take();
                    self.sampler.stop_and_join();
                    return iced::exit();
                }
            }
            Message::WindowEvent(id, ev) => match ev {
                window::Event::Opened { .. } => {
                    self.window_id = Some(id);
                    self.sync_focus_polling();
                }
                window::Event::Closed => {
                    if self.window_id == Some(id) {
                        self.window_id = None;
                        self.sync_focus_polling();
                    }
                }
                window::Event::Unfocused if self.settings.close_to_tray => {
                    return window::get_minimized(id)
                        .map(move |m| Message::CheckMinimized(id, m));
                }
                _ => {}
            },
            Message::CheckMinimized(_id, Some(true)) => {
                return self.hide_window_task();
            }
            Message::CheckMinimized(_, _) => {}
            Message::HideToTray => return self.hide_window_task(),
            Message::DismissToast => self.toast = None,
            Message::ToggleLanguage => {
                let next = if self.lang.is_pt() { Lang::En } else { Lang::Pt };
                self.save_language(next);
            }
            Message::SelectLanguage(choice) => {
                let next = if choice.starts_with("English") {
                    Lang::En
                } else {
                    Lang::Pt
                };
                self.save_language(next);
            }
            Message::SelectSection(sec) => {
                if self.section != sec {
                    self.macro_recording = false;
                    self.adv_pick_target = None;
                }
                self.section = sec;
                self.profile_menu_open = false;
                self.save_menu_open = false;
                self.sync_focus_polling();
                if sec == NavSection::AppTrigger && self.processes.is_empty() {
                    return Task::perform(
                        tokio_blocking(list_windowed_processes),
                        Message::ProcessesRefreshed,
                    );
                }
            }
            Message::ToggleKeyboardCollapsed => {
                self.keyboard_collapsed = !self.keyboard_collapsed;
            }
            Message::ToggleProfileMenu => {
                self.profile_menu_open = !self.profile_menu_open;
                self.save_menu_open = false;
            }
            Message::ToggleSaveMenu => {
                self.save_menu_open = !self.save_menu_open;
                self.profile_menu_open = false;
            }
            Message::CloseMenus => {
                self.profile_menu_open = false;
                self.save_menu_open = false;
            }
            Message::ToggleLightingRgb => {
                self.lighting_advanced_rgb = !self.lighting_advanced_rgb;
            }
            Message::KeycodeSearchChanged(q) => {
                self.keycode_search = q;
            }
            Message::ApplyPerfPreset {
                actuation,
                rt,
                press,
                release,
            } => {
                self.actuation_mm = actuation;
                self.rt_enabled = rt;
                self.rt_press_mm = press;
                self.rt_release_mm = release;
                self.ram_dirty = true;
                let indices = self.target_indices();
                self.sampler.send(DeviceCommand::SetActuation {
                    indices: indices.clone(),
                    mm: actuation,
                });
                self.sampler.send(DeviceCommand::SetRapidTrigger {
                    indices,
                    enabled: rt,
                    press_mm: press,
                    release_mm: release,
                });
            }
            Message::SelectPerfSubTab(s) => {
                self.perf_sub = s;
                self.sync_focus_polling();
            }
            Message::SelectKeyPickerTab(p) => self.pick_tab = p,
            Message::SelectAdvSubTab(a) => {
                self.adv_sub = a;
                self.adv_pick_target = None;
            }
            Message::SelectLayer(l) => self.layer = l.min(3),
            Message::Keyboard(KeyboardEvent::ClickKey { index, toggle }) => {
                let cols = self.snap.board.cols.max(1);
                let r = index / cols;
                let c = index % cols;
                if self.section == NavSection::Advanced {
                    match self.adv_pick_target {
                        Some(AdvPickTarget::Key1) => {
                            self.adv_k1_r = r.to_string();
                            self.adv_k1_c = c.to_string();
                            self.adv_pick_target = None;
                            return Task::none();
                        }
                        Some(AdvPickTarget::Key2) => {
                            self.adv_k2_r = r.to_string();
                            self.adv_k2_c = c.to_string();
                            self.adv_pick_target = None;
                            return Task::none();
                        }
                        _ => {}
                    }
                }
                if toggle {
                    if !self.selected_keys.remove(&index) {
                        self.selected_keys.insert(index);
                    }
                } else {
                    self.selected_keys.clear();
                    self.selected_keys.insert(index);
                }
                if let Some(ref cfg) = self.snap.config {
                    if let Some(&act) = cfg.actuation.get(index) {
                        self.actuation_mm = act;
                    }
                    if let Some(rt) = cfg.rapid_trigger.get(index) {
                        self.rt_enabled = rt.enabled;
                        self.rt_press_mm = rt.press_mm;
                        self.rt_release_mm = rt.release_mm;
                    }
                }
                self.sync_focus_polling();
            }
            Message::SelectKeyZone(zone) => {
                self.selected_keys = self.zone_indices(zone);
                self.sync_focus_polling();
            }
            Message::SelectAllKeys => {
                self.selected_keys = self
                    .snap
                    .populated
                    .iter()
                    .enumerate()
                    .filter(|(_, &p)| p)
                    .map(|(i, _)| i)
                    .collect();
            }
            Message::SelectNoKeys => self.selected_keys.clear(),
            Message::InvertKeys => {
                let mut next = BTreeSet::new();
                for (i, &p) in self.snap.populated.iter().enumerate() {
                    if p && !self.selected_keys.contains(&i) {
                        next.insert(i);
                    }
                }
                self.selected_keys = next;
            }
            Message::OpenFirmwareUrl => {
                let _ = open::that("https://hub.fgg.com.cn/");
            }

            // Confirmation modal
            Message::ConfirmDialogCancel => {
                self.confirm_dialog = None;
            }
            Message::ConfirmDialogAccept => {
                let Some(dlg) = self.confirm_dialog.take() else {
                    return Task::none();
                };
                match dlg.action {
                    ConfirmAction::DeleteProfile(name) => {
                        if let Ok(p) = profile_path(&profile_dir(), &name) {
                            if std::fs::remove_file(p).is_ok() {
                                self.profiles = list_profiles(&profile_dir());
                                self.selected_profile = self.profiles.first().cloned();
                                self.switcher_cfg.rules.retain(|r| r.profile != name);
                                if self.switcher_cfg.default_profile.as_deref()
                                    == Some(name.as_str())
                                {
                                    self.switcher_cfg.default_profile = None;
                                }
                                let _ = self.switcher_cfg.save(&switcher_config_path());
                                self.sampler.send(DeviceCommand::ReloadSwitcher);
                                self.sync_tray_state();
                                self.show_toast(
                                    format!("{} '{name}'", self.lang.t("Excluído:", "Deleted")),
                                    false,
                                );
                            }
                        }
                    }
                    ConfirmAction::CommitToFlash => {
                        self.sampler.send(DeviceCommand::CommitToFlash);
                        self.ram_dirty = false;
                    }
                    ConfirmAction::SaveProfileOnboard(name) => {
                        let _ = write_onboard_name(&onboard_path(), &name);
                        self.sampler.send(DeviceCommand::ApplyProfile {
                            name,
                            flash_op: FlashOp::EraseAndWrite,
                        });
                        self.ram_dirty = false;
                    }
                    ConfirmAction::StartCalibration => {
                        self.sampler.send(DeviceCommand::StartCalibration);
                    }
                    ConfirmAction::ResetKeymap => {
                        self.sampler.send(DeviceCommand::ResetKeymap);
                    }
                    ConfirmAction::UnlockBoard(pid) => {
                        self.sampler.send(DeviceCommand::UnlockBoard(pid));
                    }
                }
            }

            // Profiles
            Message::SelectProfile(name) => {
                self.selected_profile = Some(name.clone());
                self.sync_tray_state();
                self.sampler.send(DeviceCommand::ApplyProfile {
                    name,
                    flash_op: FlashOp::Normal,
                });
            }
            Message::NewProfileNameChanged(s) => self.new_profile_name = s,
            Message::CreateProfile(blank) => {
                let name = self.new_profile_name.trim().to_string();
                if name.is_empty() {
                    self.show_toast(
                        self.lang.t("Digite um nome para o perfil", "Enter a profile name first"),
                        true,
                    );
                    return Task::none();
                }
                let mut prof = Profile::blank(&name);
                if !blank {
                    self.snapshot_into_profile(&mut prof);
                }
                if let Ok(path) = profile_path(&profile_dir(), &name) {
                    match prof.save(&path) {
                        Ok(()) => {
                            self.profiles = list_profiles(&profile_dir());
                            self.selected_profile = Some(name.clone());
                            self.new_profile_name.clear();
                            self.ram_dirty = false;
                            self.sync_tray_state();
                            self.show_toast(
                                format!(
                                    "{} '{name}'",
                                    self.lang.t("Perfil criado:", "Created profile")
                                ),
                                false,
                            );
                        }
                        Err(e) => self.show_toast(e, true),
                    }
                }
            }
            Message::SaveCurrentProfile => {
                self.save_menu_open = false;
                self.profile_menu_open = false;
                self.save_current_profile();
            }
            Message::RenameProfile => {
                let Some(old) = self.selected_profile.clone() else {
                    return Task::none();
                };
                let new_name = self.new_profile_name.trim().to_string();
                if new_name.is_empty() {
                    self.show_toast(
                        self.lang.t(
                            "Digite o novo nome no campo acima",
                            "Type the new name in the input box first",
                        ),
                        true,
                    );
                    return Task::none();
                }
                if let (Ok(op), Ok(np)) = (
                    profile_path(&profile_dir(), &old),
                    profile_path(&profile_dir(), &new_name),
                ) {
                    if let Ok(mut prof) = Profile::load(&op) {
                        prof.name = new_name.clone();
                        if prof.save(&np).is_ok() {
                            let _ = std::fs::remove_file(op);
                            for rule in &mut self.switcher_cfg.rules {
                                if rule.profile == old {
                                    rule.profile = new_name.clone();
                                }
                            }
                            if self.switcher_cfg.default_profile.as_deref() == Some(old.as_str()) {
                                self.switcher_cfg.default_profile = Some(new_name.clone());
                            }
                            let _ = self.switcher_cfg.save(&switcher_config_path());
                            self.sampler.send(DeviceCommand::ReloadSwitcher);
                            self.profiles = list_profiles(&profile_dir());
                            self.selected_profile = Some(new_name);
                            self.new_profile_name.clear();
                            self.sync_tray_state();
                            self.show_toast(
                                self.lang.t("Perfil renomeado", "Profile renamed"),
                                false,
                            );
                        }
                    }
                }
            }
            Message::DuplicateProfile => {
                let Some(src) = self.selected_profile.clone() else {
                    return Task::none();
                };
                let copy_name = format!("{src} Copy");
                if let (Ok(sp), Ok(dp)) = (
                    profile_path(&profile_dir(), &src),
                    profile_path(&profile_dir(), &copy_name),
                ) {
                    if let Ok(mut prof) = Profile::load(&sp) {
                        prof.name = copy_name.clone();
                        prof.triggers.clear();
                        prof.trigger_exe = None;
                        prof.trigger_title = None;
                        if prof.save(&dp).is_ok() {
                            self.profiles = list_profiles(&profile_dir());
                            self.selected_profile = Some(copy_name);
                            self.sync_tray_state();
                            self.show_toast(
                                self.lang.t("Perfil duplicado", "Profile duplicated"),
                                false,
                            );
                        }
                    }
                }
            }
            Message::RequestDeleteProfile => {
                self.profile_menu_open = false;
                let Some(name) = self.selected_profile.clone() else {
                    return Task::none();
                };
                self.confirm_dialog = Some(ConfirmDialog {
                    title: self.lang.t("Excluir Perfil", "Delete Profile").to_string(),
                    body: format!(
                        "{} \"{name}\"? {}",
                        self.lang.t("Deseja excluir o perfil", "Permanently delete profile"),
                        self.lang.t(
                            "Regras de troca por app vinculadas a ele também serão removidas.",
                            "Any App Trigger rules pointing to it will also be removed.",
                        )
                    ),
                    confirm_label: self.lang.t("Excluir", "Delete").to_string(),
                    danger: true,
                    action: ConfirmAction::DeleteProfile(name),
                });
            }
            Message::ImportProfile => {
                self.profile_menu_open = false;
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("OpenMAD Profile", &["json"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::ImportProfilePicked,
                );
            }
            Message::ImportProfilePicked(Some(path)) => match Profile::load(&path) {
                Ok(mut prof) => {
                    let mut name = prof.name.clone();
                    let dir = profile_dir();
                    if let Ok(existing) = profile_path(&dir, &name) {
                        if existing.exists() {
                            let mut n = 2usize;
                            while profile_path(&dir, &format!("{name} ({n})"))
                                .is_ok_and(|p| p.exists())
                            {
                                n += 1;
                            }
                            name = format!("{name} ({n})");
                            prof.name = name.clone();
                        }
                    }
                    prof.triggers.clear();
                    prof.trigger_exe = None;
                    prof.trigger_title = None;
                    if let Ok(dest) = profile_path(&dir, &name) {
                        match prof.save(&dest) {
                            Ok(()) => {
                                self.profiles = list_profiles(&dir);
                                self.selected_profile = Some(name);
                                self.sync_tray_state();
                                self.show_toast(
                                    self.lang.t("Perfil importado", "Profile imported"),
                                    false,
                                );
                            }
                            Err(e) => self.show_toast(e, true),
                        }
                    }
                }
                Err(e) => self.show_toast(e, true),
            },
            Message::ImportProfilePicked(None) => {}
            Message::ExportProfile => {
                self.profile_menu_open = false;
                let Some(name) = self.selected_profile.clone() else {
                    return Task::none();
                };
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_file_name(format!("{name}.json"))
                            .add_filter("JSON", &["json"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::ExportProfilePicked,
                );
            }
            Message::ExportProfilePicked(Some(dest)) => {
                if let Some(ref name) = self.selected_profile {
                    if let Ok(src) = profile_path(&profile_dir(), name) {
                        if let Ok(prof) = Profile::load(&src) {
                            match prof.save(&dest) {
                                Ok(()) => self.show_toast(
                                    self.lang.t("Perfil exportado", "Profile exported"),
                                    false,
                                ),
                                Err(e) => self.show_toast(e, true),
                            }
                        }
                    }
                }
            }
            Message::ExportProfilePicked(None) => {}
            Message::RequestSaveProfileOnboard => {
                self.profile_menu_open = false;
                if let Some(name) = self.selected_profile.clone() {
                    self.confirm_dialog = Some(ConfirmDialog {
                        title: self
                            .lang
                            .t("Salvar Perfil na Memória Flash", "Save Profile Onboard")
                            .to_string(),
                        body: format!(
                            "{} \"{name}\" {} {}",
                            self.lang.t(
                                "Gravar o perfil",
                                "Write profile",
                            ),
                            self.lang.t(
                                "na memória interna do teclado (incluindo keymap, se salvo)?",
                                "to onboard flash memory (including its keymap, if stored)?",
                            ),
                            self.lang.t(
                                "Não desconecte o cabo USB durante a gravação.",
                                "Do not unplug USB while writing.",
                            )
                        ),
                        confirm_label: self.lang.t("Gravar na Flash", "Write to Flash").to_string(),
                        danger: false,
                        action: ConfirmAction::SaveProfileOnboard(name),
                    });
                } else {
                    return self.update(Message::RequestCommitToFlash);
                }
            }
            Message::SetDefaultProfile(name) => {
                self.switcher_cfg.default_profile =
                    if name == "(Nenhum)" || name == "(None)" {
                        None
                    } else {
                        Some(name)
                    };
                let _ = self.switcher_cfg.save(&switcher_config_path());
                self.sampler.send(DeviceCommand::ReloadSwitcher);
                self.show_toast(
                    self.lang.t(
                        "Perfil padrão de fallback atualizado",
                        "Default fallback profile updated",
                    ),
                    false,
                );
            }

            // Contextual Keycode Picker (Keys / Advanced MT & DKS / Macros)
            Message::KeycodePicked(kc) => match self.section {
                NavSection::Keys => self.assign_keycode_to_selection(kc),
                NavSection::Advanced => match self.adv_pick_target {
                    Some(AdvPickTarget::MtTap) => {
                        self.mt_tap = format!("{kc:#06x}");
                        self.adv_pick_target = None;
                    }
                    Some(AdvPickTarget::MtHold) => {
                        self.mt_hold = format!("{kc:#06x}");
                        self.adv_pick_target = None;
                    }
                    Some(AdvPickTarget::Dks(row)) if row < 4 => {
                        let st = self.dks_draft.actions[row].status;
                        self.dks_draft.actions[row] =
                            openmad_core::features::DksAction::from_keycode(kc, st);
                        self.adv_pick_target = None;
                    }
                    _ => {}
                },
                NavSection::Macros => {
                    self.macro_step_val = format!("{kc:#06x}");
                    self.macro_picker_open = false;
                }
                _ => {}
            },
            Message::CustomHexInputChanged(s) => self.custom_hex_input = s,
            Message::AssignCustomHexKeycode => {
                if let Some(kc) = parse_u16_hex_or_dec(&self.custom_hex_input) {
                    self.assign_keycode_to_selection(kc);
                } else {
                    self.show_toast(
                        self.lang.t(
                            "Código HEX inválido (ex: 0x0004)",
                            "Invalid HEX keycode (e.g. 0x0004)",
                        ),
                        true,
                    );
                }
            }
            Message::CaptureProfileKeymap => {
                let Some(name) = self.selected_profile.clone() else {
                    self.show_toast(
                        self.lang.t("Selecione um perfil primeiro", "Select a profile first"),
                        true,
                    );
                    return Task::none();
                };
                let Some(ref cfg) = self.snap.config else {
                    return Task::none();
                };
                if let Ok(path) = profile_path(&profile_dir(), &name) {
                    let mut prof = Profile::load(&path).unwrap_or_else(|_| Profile::blank(&name));
                    prof.set_keymap_bytes(
                        &cfg.keymap,
                        self.snap.board.layers,
                        self.snap.board.rows,
                        self.snap.board.cols,
                    );
                    match prof.save(&path) {
                        Ok(()) => self.show_toast(
                            format!(
                                "{} '{name}'",
                                self.lang.t("Keymap capturado no perfil", "Keymap saved into")
                            ),
                            false,
                        ),
                        Err(e) => self.show_toast(e, true),
                    }
                }
            }
            Message::ApplyProfileKeymap => {
                if let Some(name) = self.selected_profile.clone() {
                    self.sampler.send(DeviceCommand::ApplyProfileKeymap(name));
                }
            }

            // Lighting — immediate real-time dispatch
            Message::LightEffectSelected(eff) => {
                self.light_draft.effect = eff;
                if self.light_draft.speed == 0 {
                    self.light_draft.speed = 128;
                }
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetLight(self.light_draft));
            }
            Message::LightColorPreset(r, g, b) => {
                self.apply_light_color(r, g, b, true);
            }
            Message::LightHsvChanged(h, s, v) => {
                self.light_hue = h;
                self.light_sat = s;
                self.light_val = v;
                let (r, g, b) = hsv_to_rgb(h, s, v);
                self.apply_light_color(r, g, b, false);
            }
            Message::LightHexChanged(hex) => {
                self.light_hex = hex.clone();
                if let Some((r, g, b)) = parse_hex_rgb(&hex) {
                    let (h, s, v) = rgb_to_hsv(r, g, b);
                    self.light_hue = h;
                    self.light_sat = s;
                    self.light_val = v;
                    self.light_draft.r = r;
                    self.light_draft.g = g;
                    self.light_draft.b = b;
                    self.ram_dirty = true;
                    self.dispatch_color_to_device(r, g, b);
                }
            }
            Message::LightBrightnessChanged(v) => {
                self.light_draft.brightness = v.min(BRIGHTNESS_MAX);
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetLight(self.light_draft));
            }
            Message::LightSpeedChanged(v) => {
                self.light_draft.speed = v.max(1);
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetLight(self.light_draft));
            }
            Message::LightRChanged(v) => {
                self.apply_light_color(v, self.light_draft.g, self.light_draft.b, true);
            }
            Message::LightGChanged(v) => {
                self.apply_light_color(self.light_draft.r, v, self.light_draft.b, true);
            }
            Message::LightBChanged(v) => {
                self.apply_light_color(self.light_draft.r, self.light_draft.g, v, true);
            }
            Message::PaintSelectedKeys => {
                let indices = self.target_indices();
                if self.light_draft.effect != 1 {
                    self.light_draft.effect = 1;
                    self.sampler.send(DeviceCommand::SetLight(self.light_draft));
                }
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetKeyColors {
                    indices,
                    color: [self.light_draft.r, self.light_draft.g, self.light_draft.b],
                });
            }
            Message::ClearKeyColorsSelection => {
                let indices = self.target_indices();
                if self.light_draft.effect != 1 {
                    self.light_draft.effect = 1;
                    self.sampler.send(DeviceCommand::SetLight(self.light_draft));
                }
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetKeyColors {
                    indices,
                    color: [0, 0, 0],
                });
            }
            Message::BoxModeChanged(v) => {
                self.box_draft.mode = v;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetBoxLight(self.box_draft));
            }
            Message::BoxColorfulChanged(v) => {
                self.box_draft.colorful = v;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetBoxLight(self.box_draft));
            }
            Message::BoxBrightnessChanged(v) => {
                self.box_draft.brightness = v;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetBoxLight(self.box_draft));
            }
            Message::BoxSpeedChanged(v) => {
                self.box_draft.speed = v;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetBoxLight(self.box_draft));
            }

            // Performance — immediate real-time dispatch
            Message::ActuationChanged(v) => {
                self.actuation_mm = (v * 20.0).round() / 20.0;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetActuation {
                    indices: self.target_indices(),
                    mm: self.actuation_mm,
                });
            }
            Message::ApplyActuation => {
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetActuation {
                    indices: self.target_indices(),
                    mm: self.actuation_mm,
                });
                self.show_toast(
                    format!(
                        "{} {:.2} mm",
                        self.lang.t("Atuação aplicada:", "Actuation applied:"),
                        self.actuation_mm
                    ),
                    false,
                );
            }
            Message::RtEnabledToggled(v) => {
                self.rt_enabled = v;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetRapidTrigger {
                    indices: self.target_indices(),
                    enabled: self.rt_enabled,
                    press_mm: self.rt_press_mm,
                    release_mm: self.rt_release_mm,
                });
            }
            Message::RtPressChanged(v) => {
                self.rt_press_mm = (v * 20.0).round() / 20.0;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetRapidTrigger {
                    indices: self.target_indices(),
                    enabled: self.rt_enabled,
                    press_mm: self.rt_press_mm,
                    release_mm: self.rt_release_mm,
                });
            }
            Message::RtReleaseChanged(v) => {
                self.rt_release_mm = (v * 20.0).round() / 20.0;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetRapidTrigger {
                    indices: self.target_indices(),
                    enabled: self.rt_enabled,
                    press_mm: self.rt_press_mm,
                    release_mm: self.rt_release_mm,
                });
            }
            Message::ApplyRapidTrigger => {
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetRapidTrigger {
                    indices: self.target_indices(),
                    enabled: self.rt_enabled,
                    press_mm: self.rt_press_mm,
                    release_mm: self.rt_release_mm,
                });
                self.show_toast(
                    self.lang.t("Rapid Trigger aplicado", "Rapid Trigger applied"),
                    false,
                );
            }
            Message::DeadBandTopChanged(v) => {
                self.dead_top_mm = (v * 20.0).round() / 20.0;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetDeadBand(DeadBand {
                    top_mm: self.dead_top_mm,
                    bottom_mm: self.dead_bot_mm,
                }));
            }
            Message::DeadBandBottomChanged(v) => {
                self.dead_bot_mm = (v * 20.0).round() / 20.0;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetDeadBand(DeadBand {
                    top_mm: self.dead_top_mm,
                    bottom_mm: self.dead_bot_mm,
                }));
            }
            Message::ResetPeaks => self.sampler.send(DeviceCommand::ResetPeaks),
            Message::RequestStartCalibration => {
                self.confirm_dialog = Some(ConfirmDialog {
                    title: self
                        .lang
                        .t("Iniciar Calibração de Switches?", "Start Switch Calibration?")
                        .to_string(),
                    body: self
                        .lang
                        .t(
                            "A calibração redefine os limites dos sensores Hall. Pressione todas as teclas até o fim do curso e depois clique em Finalizar Calibração.",
                            "Calibration recalibrates Hall sensor endpoints. Press every key fully to bottom-out, then click Finish Calibration.",
                        )
                        .to_string(),
                    confirm_label: self.lang.t("Iniciar", "Start").to_string(),
                    danger: false,
                    action: ConfirmAction::StartCalibration,
                });
            }
            Message::FinishCalibration => self.sampler.send(DeviceCommand::FinishCalibration),
            Message::GameModeToggled(v) => {
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetGameMode(u8::from(v)));
            }
            Message::BottomOptimizeToggled(v) => {
                self.ram_dirty = true;
                self.sampler
                    .send(DeviceCommand::SetBottomOptimize(u8::from(v)));
            }
            Message::FeatureRgbAreaToggled(v) => {
                self.send_feature_update(|f| f.rgb_area = u8::from(v))
            }
            Message::FeatureWasdToggled(v) => {
                self.send_feature_update(|f| f.wasd_switch = u8::from(v))
            }
            Message::FeatureMacToggled(v) => {
                self.send_feature_update(|f| f.mac_switch = u8::from(v))
            }
            Message::FeatureWinLockToggled(v) => {
                self.send_feature_update(|f| f.win_lock = u8::from(v))
            }
            Message::FeatureNkroToggled(v) => {
                self.send_feature_update(|f| f.nkro_switch = u8::from(v))
            }

            // Advanced Key
            Message::AdvSlotSelected(i) => {
                self.adv_slot = i.min(15);
                self.adv_pick_target = None;
                if let Some(cfg) = self.snap.config.clone() {
                    self.sync_adv_slot_from_config(&cfg);
                }
            }
            Message::AdvPickTargetSelected(t) => {
                self.adv_pick_target = t;
            }
            Message::AdvModeChanged(mode) => {
                self.adv_mode = mode;
            }
            Message::AdvApcChanged(v) => self.adv_apc = (v * 20.0).round() / 20.0,
            Message::AdvRtToggled(v) => self.adv_rt = v,
            Message::SaveAdvancedKey => {
                let mode = if self.adv_sub == AdvSubTab::Socd {
                    match self.adv_mode {
                        AdvancedKeyMode::Socd
                        | AdvancedKeyMode::SocdKey1
                        | AdvancedKeyMode::SocdKey2
                        | AdvancedKeyMode::SocdBalance => self.adv_mode,
                        _ => AdvancedKeyMode::Socd,
                    }
                } else {
                    AdvancedKeyMode::Rs
                };
                let ak = AdvancedKey {
                    index: self.adv_slot,
                    mode,
                    id: 0,
                    rs_apc_lv: (self.adv_apc * 100.0).round() as u16,
                    gapc_sw: 0,
                    rt_sw: u8::from(self.adv_rt),
                    key1_row: self.adv_k1_r.parse().unwrap_or(0),
                    key1_col: self.adv_k1_c.parse().unwrap_or(0),
                    key2_row: self.adv_k2_r.parse().unwrap_or(0),
                    key2_col: self.adv_k2_c.parse().unwrap_or(0),
                    layer: self.layer as u8,
                };
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetAdvancedKey(ak));
            }
            Message::ClearAdvancedKey => {
                self.ram_dirty = true;
                self.sampler
                    .send(DeviceCommand::SetAdvancedKey(AdvancedKey::empty(self.adv_slot)));
            }
            Message::MtTimerChanged(s) => self.mt_timer = s,
            Message::SaveTapDance => {
                let td = TapDance {
                    index: self.adv_slot,
                    tap: parse_u16_hex_or_dec(&self.mt_tap).unwrap_or(0),
                    hold: parse_u16_hex_or_dec(&self.mt_hold).unwrap_or(0),
                    timer_ms: self.mt_timer.parse().unwrap_or(200),
                };
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetTapDance(td));
            }
            Message::ClearTapDance => {
                self.mt_tap = "0x0000".into();
                self.mt_hold = "0x0000".into();
                self.mt_timer = "200".into();
                self.ram_dirty = true;
                self.sampler
                    .send(DeviceCommand::SetTapDance(TapDance::empty(self.adv_slot)));
            }
            Message::DksToggleBit(row, col) => {
                if row < 4 && col < 4 {
                    self.dks_draft.actions[row].status ^= 1 << col;
                }
            }
            Message::SaveDks => {
                self.dks_draft.index = self.adv_slot;
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetDks(self.dks_draft.clone()));
            }
            Message::ClearDks => {
                self.dks_draft = Dks::empty(self.adv_slot);
                self.ram_dirty = true;
                self.sampler.send(DeviceCommand::SetDks(self.dks_draft.clone()));
            }

            // App Trigger
            Message::ToggleAutoSwitcher(v) => {
                self.switcher_cfg.enabled = v;
                let _ = self.switcher_cfg.save(&switcher_config_path());
                self.sampler.send(DeviceCommand::ReloadSwitcher);
                self.sync_tray_state();
            }
            Message::SetFallbackMode(mode) => {
                self.switcher_cfg.fallback_mode = mode;
                if mode != FallbackMode::Disabled && self.switcher_cfg.default_profile.is_none() {
                    if let Some(p) = self
                        .profiles
                        .iter()
                        .find(|p| p.eq_ignore_ascii_case("Padrão") || p.eq_ignore_ascii_case("Default"))
                    {
                        self.switcher_cfg.default_profile = Some(p.clone());
                    }
                }
                let _ = self.switcher_cfg.save(&switcher_config_path());
                self.sampler.send(DeviceCommand::ReloadSwitcher);
                let msg = match mode {
                    FallbackMode::OnClose => self.lang.t(
                        "Voltará ao perfil padrão ao fechar o jogo/app",
                        "Will revert to default profile when app closes",
                    ),
                    FallbackMode::OnUnfocus => self.lang.t(
                        "Voltará ao perfil padrão ao trocar de janela",
                        "Will revert to default profile on focus change",
                    ),
                    FallbackMode::Disabled => self.lang.t(
                        "Retorno automático ao perfil padrão desativado",
                        "Automatic fallback to default profile disabled",
                    ),
                };
                self.show_toast(msg, false);
            }
            Message::TriggerExeChanged(s) => self.trigger_exe = s,
            Message::TriggerTitleChanged(s) => self.trigger_title = s,
            Message::BrowseTriggerExe => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::TriggerExePicked,
                );
            }
            Message::TriggerExePicked(Some(path)) => {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    self.trigger_exe = name.to_string();
                }
            }
            Message::TriggerExePicked(None) => {}
            Message::AddTriggerRule => {
                let Some(prof) = self.selected_profile.clone() else {
                    self.show_toast(
                        self.lang.t(
                            "Selecione ou crie um perfil na barra lateral primeiro",
                            "Select or create a profile in the sidebar first",
                        ),
                        true,
                    );
                    return Task::none();
                };
                if self.trigger_exe.trim().is_empty() && self.trigger_title.trim().is_empty() {
                    self.show_toast(
                        self.lang.t(
                            "Informe o executável ou título da janela",
                            "Enter an executable or window title",
                        ),
                        true,
                    );
                    return Task::none();
                }
                self.switcher_cfg.rules.push(Rule {
                    profile: prof.clone(),
                    exe: self.trigger_exe.trim().to_string(),
                    title: self.trigger_title.trim().to_string(),
                });
                let _ = self.switcher_cfg.save(&switcher_config_path());
                self.sync_profile_triggers_from_switcher();
                self.sampler.send(DeviceCommand::ReloadSwitcher);
                self.trigger_exe.clear();
                self.trigger_title.clear();
                self.show_toast(
                    format!(
                        "{} '{prof}'",
                        self.lang.t("Regra vinculada ao perfil", "Trigger bound to")
                    ),
                    false,
                );
            }
            Message::RemoveTriggerRule(idx) => {
                if idx < self.switcher_cfg.rules.len() {
                    self.switcher_cfg.rules.remove(idx);
                    let _ = self.switcher_cfg.save(&switcher_config_path());
                    self.sync_profile_triggers_from_switcher();
                    self.sampler.send(DeviceCommand::ReloadSwitcher);
                }
            }
            Message::MoveTriggerRuleUp(idx) => {
                if idx > 0 && idx < self.switcher_cfg.rules.len() {
                    self.switcher_cfg.rules.swap(idx - 1, idx);
                    let _ = self.switcher_cfg.save(&switcher_config_path());
                    self.sampler.send(DeviceCommand::ReloadSwitcher);
                }
            }
            Message::MoveTriggerRuleDown(idx) => {
                if idx + 1 < self.switcher_cfg.rules.len() {
                    self.switcher_cfg.rules.swap(idx, idx + 1);
                    let _ = self.switcher_cfg.save(&switcher_config_path());
                    self.sampler.send(DeviceCommand::ReloadSwitcher);
                }
            }
            Message::SwitcherDebounceChanged(s) => {
                if let Ok(v) = s.trim().parse::<u64>() {
                    self.switcher_cfg.debounce_ms = v.min(5000);
                    let _ = self.switcher_cfg.save(&switcher_config_path());
                    self.sampler.send(DeviceCommand::ReloadSwitcher);
                }
            }
            Message::SwitcherDwellChanged(s) => {
                if let Ok(v) = s.trim().parse::<u64>() {
                    self.switcher_cfg.min_dwell_ms = v.min(10000);
                    let _ = self.switcher_cfg.save(&switcher_config_path());
                    self.sampler.send(DeviceCommand::ReloadSwitcher);
                }
            }
            Message::RefreshProcesses => {
                return Task::perform(
                    tokio_blocking(list_windowed_processes),
                    Message::ProcessesRefreshed,
                );
            }
            Message::ProcessesRefreshed(list) => {
                self.processes = list;
            }
            Message::PickRunningProcess(exe) => {
                self.trigger_exe = exe;
            }

            // System & Diagnostics
            Message::RequestUnlockBoard => {
                let pid = self.snap.board.product_id;
                let name = self.snap.board.name.clone();
                self.confirm_dialog = Some(ConfirmDialog {
                    title: self
                        .lang
                        .t("Desbloquear Escrita na Placa?", "Unlock Board Writes?")
                        .to_string(),
                    body: format!(
                        "{} {name} ({pid:#06x}) {}",
                        self.lang.t("O modelo", "Model"),
                        self.lang.t(
                            "ainda não foi validado em hardware. Faça um backup ou gere um relatório de placa antes de gravar.",
                            "is unverified on real hardware. Export a backup or run a board diagnostic report before writing.",
                        )
                    ),
                    confirm_label: self.lang.t("Desbloquear", "Unlock").to_string(),
                    danger: true,
                    action: ConfirmAction::UnlockBoard(pid),
                });
            }
            Message::LockBoard => {
                self.sampler
                    .send(DeviceCommand::LockBoard(self.snap.board.product_id));
            }
            Message::RequestCommitToFlash => {
                self.save_menu_open = false;
                self.confirm_dialog = Some(ConfirmDialog {
                    title: self
                        .lang
                        .t("Gravar na Memória Flash?", "Write to Keyboard Flash?")
                        .to_string(),
                    body: self
                        .lang
                        .t(
                            "Isso persistirá atuação, Rapid Trigger e iluminação na memória não-volátil do teclado. Não desconecte o cabo USB.",
                            "This writes actuation, Rapid Trigger, and lighting to non-volatile flash memory. Do not unplug USB.",
                        )
                        .to_string(),
                    confirm_label: self.lang.t("Gravar na Flash", "Write to Flash").to_string(),
                    danger: false,
                    action: ConfirmAction::CommitToFlash,
                });
            }
            Message::ExportBackup => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_file_name("openmad-backup.json")
                            .add_filter("JSON", &["json"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::ExportBackupPicked,
                );
            }
            Message::ExportBackupPicked(Some(path)) => {
                self.sampler.send(DeviceCommand::ExportBackup(path));
            }
            Message::ExportBackupPicked(None) => {}
            Message::RestoreBackup => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::RestoreBackupPicked,
                );
            }
            Message::RestoreBackupPicked(Some(path)) => {
                self.sampler.send(DeviceCommand::RestoreBackup {
                    path,
                    commit_flash: false,
                });
            }
            Message::RestoreBackupPicked(None) => {}
            Message::FactoryResetInputChanged(s) => self.factory_confirm = s,
            Message::ConfirmFactoryReset => {
                if self.factory_confirm.trim() == "RESET" {
                    self.factory_confirm.clear();
                    self.sampler.send(DeviceCommand::FactoryReset);
                } else {
                    self.show_toast(
                        self.lang.t(
                            "Digite RESET no campo de confirmação primeiro",
                            "Type RESET in the confirmation box first",
                        ),
                        true,
                    );
                }
            }
            Message::RequestResetKeymap => {
                self.confirm_dialog = Some(ConfirmDialog {
                    title: self
                        .lang
                        .t("Resetar Keymap na EEPROM?", "Reset Keymap in EEPROM?")
                        .to_string(),
                    body: self
                        .lang
                        .t(
                            "Restaura o mapa de teclas de fábrica em todas as camadas do teclado.",
                            "Restores the factory keymap across every layer on the keyboard.",
                        )
                        .to_string(),
                    confirm_label: self.lang.t("Resetar Keymap", "Reset Keymap").to_string(),
                    danger: true,
                    action: ConfirmAction::ResetKeymap,
                });
            }
            Message::DiagWriteProbeToggled(v) => self.diag_write_probe = v,
            Message::RunDiagnostic => {
                if self.diag_running {
                    return Task::none();
                }
                self.diag_running = true;
                let (tx, rx) = std::sync::mpsc::channel();
                self.sampler.send(DeviceCommand::RunDiagnostic {
                    write_probe: self.diag_write_probe,
                    tx,
                });
                return Task::perform(
                    tokio_blocking(move || {
                        rx.recv_timeout(Duration::from_secs(30))
                            .unwrap_or_else(|_| Err("Diagnostic timed out".to_string()))
                    }),
                    |r| Message::DiagOutputReady(Box::new(r)),
                );
            }
            Message::DiagOutputReady(res) => {
                self.diag_running = false;
                match *res {
                    Ok(out) => self.diag_output = Some(out),
                    Err(e) => self.show_toast(e, true),
                }
            }
            Message::CloseDiagModal => self.diag_output = None,
            Message::CopyDiagJson => {
                if let Some(ref diag) = self.diag_output {
                    let json = diag.json.clone();
                    self.show_toast(
                        self.lang
                            .t("JSON copiado para a área de transferência", "JSON copied to clipboard"),
                        false,
                    );
                    return iced::clipboard::write(json);
                }
            }
            Message::OpenDiagIssueUrl => {
                if let Some(ref diag) = self.diag_output {
                    let _ = open::that(&diag.issue_url);
                } else {
                    let url = format!(
                        "https://github.com/{REPO_SLUG}/issues/new?title=Board+Report:+{:#06x}",
                        self.snap.board.product_id
                    );
                    let _ = open::that(url);
                }
            }

            // Macros
            Message::SelectMacroSlot(slot) => {
                self.macro_slot = slot.min(15);
                self.macro_recording = false;
                if let Some(ref cfg) = self.snap.config {
                    self.macro_steps = cfg.macros.get(self.macro_slot).cloned().unwrap_or_default();
                }
            }
            Message::MacroStepKindChanged(k) => self.macro_step_kind = k,
            Message::MacroStepValChanged(v) => self.macro_step_val = v,
            Message::ToggleMacroPicker => self.macro_picker_open = !self.macro_picker_open,
            Message::ToggleMacroRecording => {
                self.macro_recording = !self.macro_recording;
                self.macro_rec_last = if self.macro_recording {
                    Some(Instant::now())
                } else {
                    None
                };
            }
            Message::MacroKeyboardEvent(kev) => {
                if !self.macro_recording {
                    return Task::none();
                }
                let now = Instant::now();
                let dt_ms = self
                    .macro_rec_last
                    .map(|prev| now.duration_since(prev).as_millis() as u32)
                    .unwrap_or(0);
                if let Some(step) = macro_step_from_iced_event(&kev) {
                    if !self.macro_steps.is_empty() && dt_ms > 5 {
                        self.macro_steps.push(MacroStep::Delay {
                            delay_ms: dt_ms.min(60_000),
                        });
                    }
                    self.macro_steps.push(step);
                    self.macro_rec_last = Some(now);
                }
            }
            Message::AddMacroStep => {
                let step = match self.macro_step_kind.as_str() {
                    "delay" => MacroStep::Delay {
                        delay_ms: self.macro_step_val.trim().parse().unwrap_or(50),
                    },
                    "text" => MacroStep::Text {
                        text: self.macro_step_val.clone(),
                    },
                    "down" => MacroStep::Down {
                        keycode: parse_u16_hex_or_dec(&self.macro_step_val).unwrap_or(0x04),
                    },
                    "up" => MacroStep::Up {
                        keycode: parse_u16_hex_or_dec(&self.macro_step_val).unwrap_or(0x04),
                    },
                    _ => MacroStep::Tap {
                        keycode: parse_u16_hex_or_dec(&self.macro_step_val).unwrap_or(0x04),
                    },
                };
                self.macro_steps.push(step);
            }
            Message::MoveMacroStepUp(idx) => {
                if idx > 0 && idx < self.macro_steps.len() {
                    self.macro_steps.swap(idx - 1, idx);
                }
            }
            Message::MoveMacroStepDown(idx) => {
                if idx + 1 < self.macro_steps.len() {
                    self.macro_steps.swap(idx, idx + 1);
                }
            }
            Message::RemoveMacroStep(idx) => {
                if idx < self.macro_steps.len() {
                    self.macro_steps.remove(idx);
                }
            }
            Message::SaveMacroSlot => {
                self.macro_recording = false;
                self.sampler.send(DeviceCommand::SetMacroSlot {
                    index: self.macro_slot,
                    steps: self.macro_steps.clone(),
                });
            }
            Message::ClearMacroSlot => {
                self.macro_recording = false;
                self.macro_steps.clear();
                self.sampler.send(DeviceCommand::SetMacroSlot {
                    index: self.macro_slot,
                    steps: Vec::new(),
                });
            }

            // Settings — non-blocking update check & install + live LED preview
            Message::CheckUpdates => {
                return Task::perform(tokio_blocking(check_for_update), Message::UpdateChecked);
            }
            Message::UpdateChecked(res) => {
                if let Ok(ref u) = res {
                    if u.available {
                        self.show_toast(
                            format!(
                                "{}: v{}",
                                self.lang.t("Nova versão disponível", "New version available"),
                                u.latest
                            ),
                            false,
                        );
                    }
                }
                self.update_info = Some(res);
            }
            Message::InstallUpdate => {
                if let Some(Ok(info)) = self.update_info.clone() {
                    self.update_installing = true;
                    return Task::perform(
                        tokio_blocking(move || download_and_apply_update(&info)),
                        Message::UpdateApplied,
                    );
                }
            }
            Message::UpdateApplied(res) => {
                self.update_installing = false;
                match res {
                    Ok(msg) => self.show_toast(msg, false),
                    Err(e) => {
                        if let Some(Ok(ref info)) = self.update_info {
                            let _ = open::that(&info.url);
                        }
                        self.show_toast(e, true);
                    }
                }
            }
            Message::GainRChanged(v) => {
                self.settings.led_gain[0] = (v * 100.0).round() / 100.0;
                self.sampler.send(DeviceCommand::SetGain(self.settings.led_gain));
                self.repush_preview_or_light();
            }
            Message::GainGChanged(v) => {
                self.settings.led_gain[1] = (v * 100.0).round() / 100.0;
                self.sampler.send(DeviceCommand::SetGain(self.settings.led_gain));
                self.repush_preview_or_light();
            }
            Message::GainBChanged(v) => {
                self.settings.led_gain[2] = (v * 100.0).round() / 100.0;
                self.sampler.send(DeviceCommand::SetGain(self.settings.led_gain));
                self.repush_preview_or_light();
            }
            Message::SetGainPreset(gain) => {
                self.settings.led_gain = gain;
                self.sampler.send(DeviceCommand::SetGain(gain));
                self.repush_preview_or_light();
                self.show_toast(
                    self.lang.t("Ganho RGB aplicado", "LED gain preset applied"),
                    false,
                );
            }
            Message::PreviewLedColor(rgb) => {
                if self.led_preview_restore.is_none() {
                    self.led_preview_restore = Some(self.light_draft);
                }
                let preview = LightInfo {
                    effect: 2, // Monochromatic Constant
                    speed: 128,
                    r: rgb[0],
                    g: rgb[1],
                    b: rgb[2],
                    brightness: BRIGHTNESS_MAX,
                };
                self.light_draft = preview;
                self.sampler.send(DeviceCommand::SetLight(preview));
            }
            Message::RestoreLedPreview => {
                if let Some(prev) = self.led_preview_restore.take() {
                    self.light_draft = prev;
                    self.sampler.send(DeviceCommand::SetLight(prev));
                }
            }
            Message::StartAtLoginToggled(v) => {
                self.settings.start_at_login = set_start_at_login(v);
                let _ = self.settings.save(&settings_path());
            }
            Message::CloseToTrayToggled(v) => {
                self.settings.close_to_tray = v;
                let _ = self.settings.save(&settings_path());
            }
            Message::CheckUpdatesPrefToggled(v) => {
                self.settings.check_updates = v;
                let _ = self.settings.save(&settings_path());
            }
        }
        Task::none()
    }

    fn repush_preview_or_light(&self) {
        if self.led_preview_restore.is_some() {
            self.sampler.send(DeviceCommand::SetLight(self.light_draft));
        }
    }

    fn assign_keycode_to_selection(&mut self, kc: u16) {
        if self.selected_keys.is_empty() {
            self.show_toast(
                self.lang.t(
                    "Selecione ao menos uma tecla no teclado acima",
                    "Select at least one key on the keyboard first",
                ),
                true,
            );
            return;
        }
        let cols = self.snap.board.cols.max(1);
        let positions: Vec<(usize, usize)> = self
            .selected_keys
            .iter()
            .map(|&idx| (idx / cols, idx % cols))
            .collect();
        self.sampler.send(DeviceCommand::SetKeycodes {
            layer: self.layer,
            positions,
            keycode: kc,
        });
    }

    fn zone_indices(&self, zone: KeyZone) -> BTreeSet<usize> {
        let cols = self.snap.board.cols.max(1);
        let rows = self.snap.board.rows;
        if let Some(ref cfg) = self.snap.config {
            let dec = keycodes::decode_keymap(&cfg.keymap, 0, rows, cols);
            let mut out = BTreeSet::new();
            for (r, row) in dec.iter().enumerate() {
                for (c, &kc) in row.iter().enumerate() {
                    let idx = r * cols + c;
                    if !self.snap.populated.get(idx).copied().unwrap_or(true) {
                        continue;
                    }
                    let matched = match zone {
                        KeyZone::Wasd => matches!(kc, 0x001A | 0x0004 | 0x0016 | 0x0007),
                        KeyZone::Arrows => matches!(kc, 0x004F..=0x0052),
                        KeyZone::Numbers => matches!(kc, 0x001E..=0x0027),
                        KeyZone::Modifiers => matches!(kc, 0x00E0..=0x00E7),
                    };
                    if matched {
                        out.insert(idx);
                    }
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
        let mut out = BTreeSet::new();
        match zone {
            KeyZone::Wasd => {
                for (r, c) in [(1, 2), (2, 1), (2, 2), (2, 3)] {
                    let idx = r * cols + c;
                    if idx < self.snap.populated.len() && self.snap.populated[idx] {
                        out.insert(idx);
                    }
                }
            }
            KeyZone::Numbers => {
                for c in 1..=10 {
                    let idx = c;
                    if idx < self.snap.populated.len() && self.snap.populated[idx] {
                        out.insert(idx);
                    }
                }
            }
            KeyZone::Arrows | KeyZone::Modifiers => {}
        }
        out
    }

    fn send_feature_update(&mut self, mutate: impl FnOnce(&mut KeyboardFeature)) {
        if let Some(ref cfg) = self.snap.config {
            let mut f = cfg.feature;
            mutate(&mut f);
            self.ram_dirty = true;
            self.sampler.send(DeviceCommand::SetFeature(f));
        }
    }

    fn apply_light_color(&mut self, r: u8, g: u8, b: u8, update_hsv: bool) {
        self.light_draft.r = r;
        self.light_draft.g = g;
        self.light_draft.b = b;
        self.light_hex = format!("#{r:02X}{g:02X}{b:02X}");
        if update_hsv {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            self.light_hue = h;
            self.light_sat = s;
            self.light_val = v;
        }
        self.ram_dirty = true;
        self.dispatch_color_to_device(r, g, b);
    }

    fn dispatch_color_to_device(&mut self, r: u8, g: u8, b: u8) {
        if self.light_draft.effect == 1 {
            self.sampler.send(DeviceCommand::SetKeyColors {
                indices: self.target_indices(),
                color: [r, g, b],
            });
            return;
        }
        let supports_color = LIGHT_EFFECTS
            .iter()
            .find(|e| e.id == self.light_draft.effect)
            .is_none_or(|e| e.has_color);
        if !supports_color {
            self.light_draft.effect = 2;
        }
        self.sampler.send(DeviceCommand::SetLight(self.light_draft));
    }

    pub fn accent(&self) -> Color {
        dynamic_accent([self.light_draft.r, self.light_draft.g, self.light_draft.b])
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.ram_dirty
    }

    pub fn view(&self, _window: window::Id) -> Element<'_, Message> {
        self.view_shell()
    }
}

async fn tokio_blocking<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.await.expect("worker thread panicked")
}

fn macro_step_from_iced_event(kev: &iced::keyboard::Event) -> Option<MacroStep> {
    match kev {
        iced::keyboard::Event::KeyPressed {
            physical_key,
            key,
            location,
            ..
        } => {
            let kc = match physical_key {
                key::Physical::Code(c) => qmk_from_physical_code(*c),
                _ => None,
            }
            .or_else(|| qmk_from_logical_key(key, *location))?;
            Some(MacroStep::Down { keycode: kc })
        }
        iced::keyboard::Event::KeyReleased { key, location, .. } => {
            let kc = qmk_from_logical_key(key, *location)?;
            Some(MacroStep::Up { keycode: kc })
        }
        iced::keyboard::Event::ModifiersChanged(_) => None,
    }
}

fn qmk_from_physical_code(code: key::Code) -> Option<u16> {
    use key::Code;
    Some(match code {
        Code::KeyA => 0x0004,
        Code::KeyB => 0x0005,
        Code::KeyC => 0x0006,
        Code::KeyD => 0x0007,
        Code::KeyE => 0x0008,
        Code::KeyF => 0x0009,
        Code::KeyG => 0x000A,
        Code::KeyH => 0x000B,
        Code::KeyI => 0x000C,
        Code::KeyJ => 0x000D,
        Code::KeyK => 0x000E,
        Code::KeyL => 0x000F,
        Code::KeyM => 0x0010,
        Code::KeyN => 0x0011,
        Code::KeyO => 0x0012,
        Code::KeyP => 0x0013,
        Code::KeyQ => 0x0014,
        Code::KeyR => 0x0015,
        Code::KeyS => 0x0016,
        Code::KeyT => 0x0017,
        Code::KeyU => 0x0018,
        Code::KeyV => 0x0019,
        Code::KeyW => 0x001A,
        Code::KeyX => 0x001B,
        Code::KeyY => 0x001C,
        Code::KeyZ => 0x001D,
        Code::Digit1 => 0x001E,
        Code::Digit2 => 0x001F,
        Code::Digit3 => 0x0020,
        Code::Digit4 => 0x0021,
        Code::Digit5 => 0x0022,
        Code::Digit6 => 0x0023,
        Code::Digit7 => 0x0024,
        Code::Digit8 => 0x0025,
        Code::Digit9 => 0x0026,
        Code::Digit0 => 0x0027,
        Code::Enter => 0x0028,
        Code::Escape => 0x0029,
        Code::Backspace => 0x002A,
        Code::Tab => 0x002B,
        Code::Space => 0x002C,
        Code::Minus => 0x002D,
        Code::Equal => 0x002E,
        Code::BracketLeft => 0x002F,
        Code::BracketRight => 0x0030,
        Code::Backslash => 0x0031,
        Code::Semicolon => 0x0033,
        Code::Quote => 0x0034,
        Code::Backquote => 0x0035,
        Code::Comma => 0x0036,
        Code::Period => 0x0037,
        Code::Slash => 0x0038,
        Code::CapsLock => 0x0039,
        Code::F1 => 0x003A,
        Code::F2 => 0x003B,
        Code::F3 => 0x003C,
        Code::F4 => 0x003D,
        Code::F5 => 0x003E,
        Code::F6 => 0x003F,
        Code::F7 => 0x0040,
        Code::F8 => 0x0041,
        Code::F9 => 0x0042,
        Code::F10 => 0x0043,
        Code::F11 => 0x0044,
        Code::F12 => 0x0045,
        Code::PrintScreen => 0x0046,
        Code::ScrollLock => 0x0047,
        Code::Pause => 0x0048,
        Code::Insert => 0x0049,
        Code::Home => 0x004A,
        Code::PageUp => 0x004B,
        Code::Delete => 0x004C,
        Code::End => 0x004D,
        Code::PageDown => 0x004E,
        Code::ArrowRight => 0x004F,
        Code::ArrowLeft => 0x0050,
        Code::ArrowDown => 0x0051,
        Code::ArrowUp => 0x0052,
        Code::NumLock => 0x0053,
        Code::NumpadDivide => 0x0054,
        Code::NumpadMultiply => 0x0055,
        Code::NumpadSubtract => 0x0056,
        Code::NumpadAdd => 0x0057,
        Code::NumpadEnter => 0x0058,
        Code::Numpad1 => 0x0059,
        Code::Numpad2 => 0x005A,
        Code::Numpad3 => 0x005B,
        Code::Numpad4 => 0x005C,
        Code::Numpad5 => 0x005D,
        Code::Numpad6 => 0x005E,
        Code::Numpad7 => 0x005F,
        Code::Numpad8 => 0x0060,
        Code::Numpad9 => 0x0061,
        Code::Numpad0 => 0x0062,
        Code::NumpadDecimal => 0x0063,
        Code::IntlBackslash => 0x0064,
        Code::ContextMenu => 0x0065,
        Code::ControlLeft => 0x00E0,
        Code::ShiftLeft => 0x00E1,
        Code::AltLeft => 0x00E2,
        Code::SuperLeft => 0x00E3,
        Code::ControlRight => 0x00E4,
        Code::ShiftRight => 0x00E5,
        Code::AltRight => 0x00E6,
        Code::SuperRight => 0x00E7,
        _ => return None,
    })
}

fn qmk_from_logical_key(key: &Key, loc: Location) -> Option<u16> {
    match key {
        Key::Character(s) => {
            let ch = s.chars().next()?.to_ascii_uppercase();
            Some(match ch {
                'A'..='Z' => 0x0004 + (ch as u16 - 'A' as u16),
                '1'..='9' => 0x001E + (ch as u16 - '1' as u16),
                '0' => 0x0027,
                ' ' => 0x002C,
                '-' => 0x002D,
                '=' => 0x002E,
                '[' => 0x002F,
                ']' => 0x0030,
                '\\' => 0x0031,
                ';' => 0x0033,
                '\'' => 0x0034,
                '`' => 0x0035,
                ',' => 0x0036,
                '.' => 0x0037,
                '/' => 0x0038,
                _ => return None,
            })
        }
        Key::Named(n) => {
            use key::Named;
            Some(match n {
                Named::Enter => 0x0028,
                Named::Escape => 0x0029,
                Named::Backspace => 0x002A,
                Named::Tab => 0x002B,
                Named::Space => 0x002C,
                Named::CapsLock => 0x0039,
                Named::F1 => 0x003A,
                Named::F2 => 0x003B,
                Named::F3 => 0x003C,
                Named::F4 => 0x003D,
                Named::F5 => 0x003E,
                Named::F6 => 0x003F,
                Named::F7 => 0x0040,
                Named::F8 => 0x0041,
                Named::F9 => 0x0042,
                Named::F10 => 0x0043,
                Named::F11 => 0x0044,
                Named::F12 => 0x0045,
                Named::Insert => 0x0049,
                Named::Home => 0x004A,
                Named::PageUp => 0x004B,
                Named::Delete => 0x004C,
                Named::End => 0x004D,
                Named::PageDown => 0x004E,
                Named::ArrowRight => 0x004F,
                Named::ArrowLeft => 0x0050,
                Named::ArrowDown => 0x0051,
                Named::ArrowUp => 0x0052,
                Named::Control => {
                    if loc == Location::Right {
                        0x00E4
                    } else {
                        0x00E0
                    }
                }
                Named::Shift => {
                    if loc == Location::Right {
                        0x00E5
                    } else {
                        0x00E1
                    }
                }
                Named::Alt => {
                    if loc == Location::Right {
                        0x00E6
                    } else {
                        0x00E2
                    }
                }
                Named::Super => {
                    if loc == Location::Right {
                        0x00E7
                    } else {
                        0x00E3
                    }
                }
                _ => return None,
            })
        }
        Key::Unidentified => None,
    }
}

pub(crate) fn parse_u16_hex_or_dec(s: &str) -> Option<u16> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u16::from_str_radix(hex, 16).ok()
    } else {
        t.parse::<u16>().ok()
    }
}

pub(crate) fn parse_hex_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 || !t.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&t[0..2], 16).ok()?;
    let g = u8::from_str_radix(&t[2..4], 16).ok()?;
    let b = u8::from_str_radix(&t[4..6], 16).ok()?;
    Some((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsers_and_physical_mapping() {
        assert_eq!(parse_u16_hex_or_dec("0x0004"), Some(4));
        assert_eq!(parse_u16_hex_or_dec("200"), Some(200));
        assert_eq!(parse_hex_rgb("#0A84FF"), Some((10, 132, 255)));
        assert_eq!(parse_hex_rgb("invalid"), None);
        assert_eq!(qmk_from_physical_code(key::Code::KeyA), Some(0x0004));
        assert_eq!(qmk_from_physical_code(key::Code::ShiftRight), Some(0x00E5));
    }
}
