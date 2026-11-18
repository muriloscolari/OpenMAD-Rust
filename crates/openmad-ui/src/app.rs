//! Main OpenMAD-Rust iced application state, command dispatch, and subscriptions.

use crate::i18n::Lang;
use crate::theme::dynamic_accent;
use crate::tray::{TrayAction, TrayHandle};
use crate::widgets::color_picker::{hsv_to_rgb, rgb_to_hsv};
use crate::widgets::keyboard::KeyboardEvent;
use iced::{time, window, Color, Element, Subscription, Task};
use openmad_core::device::FullConfig;
use openmad_core::features::{
    AdvancedKey, BoxLight, DeadBand, Dks, DksAction, KeyboardFeature, LightInfo, MacroStep,
    TapDance, LIGHT_EFFECTS,
};
use openmad_core::paths::{onboard_path, profile_dir, settings_path, switcher_config_path};
use openmad_core::profile::{
    key_str, list_profiles, profile_path, write_onboard_name, Profile, RapidTriggerSpec,
};
use openmad_core::protocol::{AdvancedKeyMode, BRIGHTNESS_MAX, FlashOp};
use openmad_core::sampler::{DeviceCommand, SamplerHandle, Snapshot};
use openmad_core::settings::{set_start_at_login, AppSettings};
use openmad_core::switcher::{
    list_windowed_processes, Rule, SwitcherConfig, WindowedProcess,
};
use openmad_core::updater::{
    check_for_update, download_and_apply_update, UpdateInfo, REPO_SLUG,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
    SelectAllKeys,
    SelectNoKeys,
    InvertKeys,
    OpenFirmwareUrl,

    // Profile management
    SelectProfile(String),
    NewProfileNameChanged(String),
    CreateProfile(bool),
    SaveCurrentProfile,
    RenameProfile,
    DuplicateProfile,
    DeleteProfile,
    ImportProfile,
    ExportProfile,
    SaveProfileOnboard,
    SetDefaultProfile(String),

    // Keymap
    AssignKeycode(u16),
    ApplyFullKeymap,

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
    SaveLighting,
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
    StartCalibration,
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
    AdvKey1RowChanged(String),
    AdvKey1ColChanged(String),
    AdvKey2RowChanged(String),
    AdvKey2ColChanged(String),
    AdvApcChanged(f32),
    AdvRtToggled(bool),
    SaveAdvancedKey,
    ClearAdvancedKey,
    MtTapChanged(String),
    MtHoldChanged(String),
    MtTimerChanged(String),
    SaveTapDance,
    DksActionChanged(usize, String, String),
    SaveDks,

    // App Trigger
    ToggleAutoSwitcher(bool),
    TriggerExeChanged(String),
    TriggerTitleChanged(String),
    AddTriggerRule,
    RemoveTriggerRule(usize),
    RefreshProcesses,
    PickRunningProcess(String),

    // Other Settings
    UnlockBoard,
    CommitToFlash,
    ExportBackup,
    RestoreBackup,
    FactoryResetInputChanged(String),
    ConfirmFactoryReset,
    OpenGithubIssue,

    // Macros
    SelectMacroSlot(usize),
    MacroStepKindChanged(String),
    MacroStepValChanged(String),
    AddMacroStep,
    RemoveMacroStep(usize),
    SaveMacroSlot,
    ClearMacroSlot,

    // Settings
    CheckUpdates,
    InstallUpdate,
    GainRChanged(f32),
    GainGChanged(f32),
    GainBChanged(f32),
    SetGainPreset([f32; 3]),
    StartAtLoginToggled(bool),
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

    pub(crate) factory_confirm: String,
    pub(crate) settings: AppSettings,
    pub(crate) update_info: Option<Result<UpdateInfo, String>>,
    pub(crate) pending_update: Arc<Mutex<Option<Result<UpdateInfo, String>>>>,
    pub(crate) toast: Option<(String, bool, u8)>,
    pub(crate) synced_initial: bool,
    pub(crate) screenshot_target: Option<String>,
    pub(crate) screenshot_ticks: u32,
}

impl OpenMadApp {
    pub fn new() -> (Self, Task<Message>) {
        let sampler = SamplerHandle::start();
        let profiles = list_profiles(&profile_dir());
        let tray = TrayHandle::new(&profiles);
        let snap = sampler.read_snapshot();
        let selected_profile = profiles.first().cloned();
        let settings = AppSettings::load(&settings_path());
        let lang = Lang::from_code(&settings.language);
        let switcher_cfg = SwitcherConfig::load(&switcher_config_path());
        let processes = list_windowed_processes();

        let start_minimized = std::env::args().any(|a| a == "--minimized" || a == "--tray");
        let screenshot_target = std::env::args()
            .find_map(|a| a.strip_prefix("--screenshot=").map(|s| s.to_string()));
        let (window_id, init_task) = if start_minimized && screenshot_target.is_none() {
            (None, Task::none())
        } else {
            let (win_id, open_task) = window::open(crate::window_settings());
            (Some(win_id), open_task.discard())
        };

        let pending_update = Arc::new(Mutex::new(None));
        if settings.check_updates {
            let slot = Arc::clone(&pending_update);
            std::thread::spawn(move || {
                let res = check_for_update();
                if let Ok(mut g) = slot.lock() {
                    *g = Some(res);
                }
            });
        }

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
            factory_confirm: String::new(),
            settings,
            update_info: None,
            pending_update,
            toast: None,
            synced_initial: false,
            screenshot_target,
            screenshot_ticks: 0,
        };
        (app, init_task)
    }

    pub fn title(&self, _window: window::Id) -> String {
        if self.snap.connected {
            format!("OpenMAD-Rust — {}", self.snap.board.name)
        } else {
            "OpenMAD-Rust".to_string()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            time::every(Duration::from_millis(33)).map(|_| Message::Tick),
            window::close_requests().map(Message::WindowCloseRequested),
            window::events().map(|(id, ev)| Message::WindowEvent(id, ev)),
        ])
    }

    fn show_window_task(&mut self) -> Task<Message> {
        if let Some(id) = self.window_id {
            Task::batch([window::minimize(id, false), window::gain_focus(id)])
        } else {
            let (id, open_task) = window::open(crate::window_settings());
            self.window_id = Some(id);
            open_task.discard()
        }
    }

    fn hide_window_task(&mut self) -> Task<Message> {
        if let Some(id) = self.window_id.take() {
            window::close(id)
        } else {
            Task::none()
        }
    }

    fn sync_tray_profiles(&self) {
        if let Some(ref tray) = self.tray {
            tray.update_profiles(&self.profiles);
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

            let cols = self.snap.board.cols;
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

        match prof.save(&path) {
            Ok(()) => self.show_toast(
                format!(
                    "{} '{name}'",
                    self.lang.t("Perfil salvo:", "Saved profile")
                ),
                false,
            ),
            Err(e) => self.show_toast(e, true),
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
        if let Some(&act) = cfg.actuation.first() {
            self.actuation_mm = act;
        }
        if let Some(rt) = cfg.rapid_trigger.first() {
            self.rt_enabled = rt.enabled;
            self.rt_press_mm = rt.press_mm;
            self.rt_release_mm = rt.release_mm;
        }
        if let Some(m) = cfg.macros.get(self.macro_slot) {
            self.macro_steps = m.clone();
        }
    }

    fn save_language(&mut self, lang: Lang) {
        self.lang = lang;
        self.settings.language = lang.code().to_string();
        let _ = self.settings.save(&settings_path());
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Tick => {
                self.snap = self.sampler.read_snapshot();
                if !self.synced_initial {
                    if let Some(cfg) = self.snap.config.clone() {
                        self.sync_drafts_from_config(&cfg);
                        self.synced_initial = true;
                    }
                }
                if let Some((m, err)) = self.sampler.take_toast() {
                    self.show_toast(m, err);
                }
                let upd = self.pending_update.lock().ok().and_then(|mut g| g.take());
                if let Some(res) = upd {
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
                            }
                            TrayAction::ApplyProfile(name) => {
                                self.selected_profile = Some(name.clone());
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
                        "lighting" => {
                            self.section = NavSection::Lighting;
                        }
                        "performance" => {
                            self.section = NavSection::Performance;
                            self.perf_sub = PerfSubTab::Travel;
                        }
                        "app-trigger" => {
                            self.section = NavSection::AppTrigger;
                        }
                        "remap" => {
                            self.section = NavSection::Keys;
                        }
                        "settings" => {
                            self.section = NavSection::Settings;
                        }
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
            Message::WindowCloseRequested(id) => {
                if self.window_id == Some(id) {
                    self.window_id = None;
                }
                return window::close(id);
            }
            Message::WindowEvent(id, ev) => {
                match ev {
                    window::Event::Opened { .. } => {
                        self.window_id = Some(id);
                    }
                    window::Event::Closed => {
                        if self.window_id == Some(id) {
                            self.window_id = None;
                        }
                    }
                    window::Event::Unfocused => {
                        return window::get_minimized(id)
                            .map(move |m| Message::CheckMinimized(id, m));
                    }
                    _ => {}
                }
            }
            Message::CheckMinimized(id, Some(true)) => {
                if self.window_id == Some(id) {
                    self.window_id = None;
                }
                return window::close(id);
            }
            Message::CheckMinimized(_, _) => {}
            Message::HideToTray => {
                return self.hide_window_task();
            }
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
                self.section = sec;
                self.profile_menu_open = false;
                self.save_menu_open = false;
                if sec == NavSection::AppTrigger && self.processes.is_empty() {
                    self.processes = list_windowed_processes();
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
            Message::SelectPerfSubTab(s) => self.perf_sub = s,
            Message::SelectKeyPickerTab(p) => self.pick_tab = p,
            Message::SelectAdvSubTab(a) => self.adv_sub = a,
            Message::SelectLayer(l) => self.layer = l.min(3),
            Message::Keyboard(KeyboardEvent::ClickKey { index, toggle }) => {
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

            // Profiles
            Message::SelectProfile(name) => {
                self.selected_profile = Some(name.clone());
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
                    if let Some(ref cfg) = self.snap.config {
                        prof.light = Some(cfg.light);
                        prof.box_light = Some(cfg.box_light);
                        prof.dead_band = Some(cfg.dead_band);
                        prof.feature = Some(cfg.feature);
                        prof.game_mode = Some(cfg.game_mode);
                        prof.bottom_optimize = Some(cfg.bottom_optimize);
                        prof.key_colors = Some(cfg.key_colors.clone());
                        if let Some(&first_act) = cfg.actuation.first() {
                            prof.actuation_default = Some(first_act);
                        }
                    }
                }
                if let Ok(path) = profile_path(&profile_dir(), &name) {
                    match prof.save(&path) {
                        Ok(()) => {
                            self.profiles = list_profiles(&profile_dir());
                            self.selected_profile = Some(name.clone());
                            self.new_profile_name.clear();
                            self.sync_tray_profiles();
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
                            self.profiles = list_profiles(&profile_dir());
                            self.selected_profile = Some(new_name);
                            self.new_profile_name.clear();
                            self.sync_tray_profiles();
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
                        if prof.save(&dp).is_ok() {
                            self.profiles = list_profiles(&profile_dir());
                            self.selected_profile = Some(copy_name);
                            self.sync_tray_profiles();
                            self.show_toast(
                                self.lang.t("Perfil duplicado", "Profile duplicated"),
                                false,
                            );
                        }
                    }
                }
            }
            Message::DeleteProfile => {
                let Some(name) = self.selected_profile.clone() else {
                    return Task::none();
                };
                if let Ok(p) = profile_path(&profile_dir(), &name) {
                    if std::fs::remove_file(p).is_ok() {
                        self.profiles = list_profiles(&profile_dir());
                        self.selected_profile = self.profiles.first().cloned();
                        self.sync_tray_profiles();
                        self.show_toast(
                            format!("{} '{name}'", self.lang.t("Excluído:", "Deleted")),
                            false,
                        );
                    }
                }
            }
            Message::ImportProfile => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("OpenMAD Profile", &["json"])
                    .pick_file()
                {
                    match Profile::load(&path) {
                        Ok(prof) => {
                            let name = prof.name.clone();
                            if let Ok(dest) = profile_path(&profile_dir(), &name) {
                                if prof.save(&dest).is_ok() {
                                    self.profiles = list_profiles(&profile_dir());
                                    self.selected_profile = Some(name);
                                    self.sync_tray_profiles();
                                    self.show_toast(
                                        self.lang.t("Perfil importado", "Profile imported"),
                                        false,
                                    );
                                }
                            }
                        }
                        Err(e) => self.show_toast(e, true),
                    }
                }
            }
            Message::ExportProfile => {
                let Some(name) = self.selected_profile.clone() else {
                    return Task::none();
                };
                if let Ok(src) = profile_path(&profile_dir(), &name) {
                    if let Ok(prof) = Profile::load(&src) {
                        if let Some(dest) = rfd::FileDialog::new()
                            .set_file_name(format!("{name}.json"))
                            .add_filter("JSON", &["json"])
                            .save_file()
                        {
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
            Message::SaveProfileOnboard => {
                if let Some(name) = self.selected_profile.clone() {
                    let _ = write_onboard_name(&onboard_path(), &name);
                    self.sampler.send(DeviceCommand::ApplyProfile {
                        name,
                        flash_op: FlashOp::EraseAndWrite,
                    });
                } else {
                    self.sampler.send(DeviceCommand::CommitToFlash);
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

            // Keymap
            Message::AssignKeycode(kc) => {
                if self.selected_keys.is_empty() {
                    self.show_toast(
                        self.lang.t(
                            "Selecione ao menos uma tecla no teclado acima",
                            "Select at least one key on the keyboard first",
                        ),
                        true,
                    );
                    return Task::none();
                }
                let cols = self.snap.board.cols;
                for &idx in &self.selected_keys {
                    self.sampler.send(DeviceCommand::SetKeycode {
                        layer: self.layer,
                        row: idx / cols,
                        col: idx % cols,
                        keycode: kc,
                    });
                }
            }
            Message::ApplyFullKeymap => {
                if let Some(ref cfg) = self.snap.config {
                    self.sampler.send(DeviceCommand::ApplyKeymap(cfg.keymap.clone()));
                }
            }

            // Lighting — immediate real-time dispatch!
            Message::LightEffectSelected(eff) => {
                self.light_draft.effect = eff;
                if self.light_draft.speed == 0 {
                    self.light_draft.speed = 128;
                }
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
                    self.dispatch_color_to_device(r, g, b);
                }
            }
            Message::LightBrightnessChanged(v) => {
                self.light_draft.brightness = v.min(BRIGHTNESS_MAX);
                self.sampler.send(DeviceCommand::SetLight(self.light_draft));
            }
            Message::LightSpeedChanged(v) => {
                self.light_draft.speed = v.max(1);
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
                if self.selected_keys.is_empty() {
                    self.show_toast(
                        self.lang.t("Selecione teclas para pintar", "Select keys to paint"),
                        true,
                    );
                    return Task::none();
                }
                if self.light_draft.effect != 1 {
                    self.light_draft.effect = 1;
                    self.sampler.send(DeviceCommand::SetLight(self.light_draft));
                }
                self.sampler.send(DeviceCommand::SetKeyColors {
                    indices: self.selected_keys.iter().copied().collect(),
                    color: [self.light_draft.r, self.light_draft.g, self.light_draft.b],
                });
            }
            Message::SaveLighting => self.sampler.send(DeviceCommand::CommitToFlash),
            Message::BoxModeChanged(v) => {
                self.box_draft.mode = v;
                self.sampler.send(DeviceCommand::SetBoxLight(self.box_draft));
            }
            Message::BoxColorfulChanged(v) => {
                self.box_draft.colorful = v;
                self.sampler.send(DeviceCommand::SetBoxLight(self.box_draft));
            }
            Message::BoxBrightnessChanged(v) => {
                self.box_draft.brightness = v;
                self.sampler.send(DeviceCommand::SetBoxLight(self.box_draft));
            }
            Message::BoxSpeedChanged(v) => {
                self.box_draft.speed = v;
                self.sampler.send(DeviceCommand::SetBoxLight(self.box_draft));
            }

            // Performance — immediate real-time dispatch!
            Message::ActuationChanged(v) => {
                self.actuation_mm = (v * 20.0).round() / 20.0;
                self.sampler.send(DeviceCommand::SetActuation {
                    indices: self.target_indices(),
                    mm: self.actuation_mm,
                });
            }
            Message::ApplyActuation => {
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
                self.sampler.send(DeviceCommand::SetRapidTrigger {
                    indices: self.target_indices(),
                    enabled: self.rt_enabled,
                    press_mm: self.rt_press_mm,
                    release_mm: self.rt_release_mm,
                });
            }
            Message::RtPressChanged(v) => {
                self.rt_press_mm = (v * 20.0).round() / 20.0;
                self.sampler.send(DeviceCommand::SetRapidTrigger {
                    indices: self.target_indices(),
                    enabled: self.rt_enabled,
                    press_mm: self.rt_press_mm,
                    release_mm: self.rt_release_mm,
                });
            }
            Message::RtReleaseChanged(v) => {
                self.rt_release_mm = (v * 20.0).round() / 20.0;
                self.sampler.send(DeviceCommand::SetRapidTrigger {
                    indices: self.target_indices(),
                    enabled: self.rt_enabled,
                    press_mm: self.rt_press_mm,
                    release_mm: self.rt_release_mm,
                });
            }
            Message::ApplyRapidTrigger => {
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
                self.sampler.send(DeviceCommand::SetDeadBand(DeadBand {
                    top_mm: self.dead_top_mm,
                    bottom_mm: self.dead_bot_mm,
                }));
            }
            Message::DeadBandBottomChanged(v) => {
                self.dead_bot_mm = (v * 20.0).round() / 20.0;
                self.sampler.send(DeviceCommand::SetDeadBand(DeadBand {
                    top_mm: self.dead_top_mm,
                    bottom_mm: self.dead_bot_mm,
                }));
            }
            Message::ResetPeaks => self.sampler.send(DeviceCommand::ResetPeaks),
            Message::StartCalibration => self.sampler.send(DeviceCommand::StartCalibration),
            Message::FinishCalibration => self.sampler.send(DeviceCommand::FinishCalibration),
            Message::GameModeToggled(v) => {
                self.sampler.send(DeviceCommand::SetGameMode(u8::from(v)));
            }
            Message::BottomOptimizeToggled(v) => {
                self.sampler.send(DeviceCommand::SetBottomOptimize(u8::from(v)));
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
                if let Some(ref cfg) = self.snap.config {
                    if let Some(ak) = cfg.advanced_keys.get(self.adv_slot) {
                        self.adv_k1_r = ak.key1_row.to_string();
                        self.adv_k1_c = ak.key1_col.to_string();
                        self.adv_k2_r = ak.key2_row.to_string();
                        self.adv_k2_c = ak.key2_col.to_string();
                        self.adv_apc = (ak.rs_apc_lv as f32) * 0.01;
                        self.adv_rt = ak.rt_sw != 0;
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
            }
            Message::AdvKey1RowChanged(s) => self.adv_k1_r = s,
            Message::AdvKey1ColChanged(s) => self.adv_k1_c = s,
            Message::AdvKey2RowChanged(s) => self.adv_k2_r = s,
            Message::AdvKey2ColChanged(s) => self.adv_k2_c = s,
            Message::AdvApcChanged(v) => self.adv_apc = (v * 20.0).round() / 20.0,
            Message::AdvRtToggled(v) => self.adv_rt = v,
            Message::SaveAdvancedKey => {
                let mode = if self.adv_sub == AdvSubTab::Socd {
                    AdvancedKeyMode::Socd
                } else {
                    AdvancedKeyMode::Rs
                };
                let ak = AdvancedKey {
                    index: self.adv_slot,
                    mode,
                    id: self.adv_slot as u8,
                    rs_apc_lv: (self.adv_apc * 100.0).round() as u16,
                    gapc_sw: 1,
                    rt_sw: u8::from(self.adv_rt),
                    key1_row: self.adv_k1_r.parse().unwrap_or(0),
                    key1_col: self.adv_k1_c.parse().unwrap_or(0),
                    key2_row: self.adv_k2_r.parse().unwrap_or(0),
                    key2_col: self.adv_k2_c.parse().unwrap_or(0),
                    layer: self.layer as u8,
                };
                self.sampler.send(DeviceCommand::SetAdvancedKey(ak));
            }
            Message::ClearAdvancedKey => {
                self.sampler
                    .send(DeviceCommand::SetAdvancedKey(AdvancedKey::empty(self.adv_slot)));
            }
            Message::MtTapChanged(s) => self.mt_tap = s,
            Message::MtHoldChanged(s) => self.mt_hold = s,
            Message::MtTimerChanged(s) => self.mt_timer = s,
            Message::SaveTapDance => {
                let td = TapDance {
                    index: self.adv_slot,
                    tap: parse_u16_hex_or_dec(&self.mt_tap).unwrap_or(0),
                    hold: parse_u16_hex_or_dec(&self.mt_hold).unwrap_or(0),
                    timer_ms: self.mt_timer.parse().unwrap_or(200),
                };
                self.sampler.send(DeviceCommand::SetTapDance(td));
            }
            Message::DksActionChanged(idx, kc_str, status_str) => {
                if idx < 4 {
                    let kc = parse_u16_hex_or_dec(&kc_str).unwrap_or(0);
                    let st = parse_u16_hex_or_dec(&status_str).unwrap_or(0);
                    self.dks_draft.actions[idx] = DksAction::from_keycode(kc, st);
                }
            }
            Message::SaveDks => {
                self.dks_draft.index = self.adv_slot;
                self.sampler.send(DeviceCommand::SetDks(self.dks_draft.clone()));
            }

            // App Trigger
            Message::ToggleAutoSwitcher(v) => {
                self.switcher_cfg.enabled = v;
                let _ = self.switcher_cfg.save(&switcher_config_path());
                self.sampler.send(DeviceCommand::ReloadSwitcher);
            }
            Message::TriggerExeChanged(s) => self.trigger_exe = s,
            Message::TriggerTitleChanged(s) => self.trigger_title = s,
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
                    self.sampler.send(DeviceCommand::ReloadSwitcher);
                }
            }
            Message::RefreshProcesses => {
                self.processes = list_windowed_processes();
            }
            Message::PickRunningProcess(exe) => {
                self.trigger_exe = exe;
            }

            // Other Settings
            Message::UnlockBoard => {
                self.sampler
                    .send(DeviceCommand::UnlockBoard(self.snap.board.product_id));
            }
            Message::CommitToFlash => {
                self.save_menu_open = false;
                self.sampler.send(DeviceCommand::CommitToFlash);
            }
            Message::ExportBackup => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("openmad-backup.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    self.sampler.send(DeviceCommand::ExportBackup(path));
                }
            }
            Message::RestoreBackup => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    self.sampler.send(DeviceCommand::RestoreBackup {
                        path,
                        commit_flash: false,
                    });
                }
            }
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
            Message::OpenGithubIssue => {
                let url = format!(
                    "https://github.com/{REPO_SLUG}/issues/new?title=Board+Report:+{:#06x}",
                    self.snap.board.product_id
                );
                let _ = open::that(url);
            }

            // Macros
            Message::SelectMacroSlot(slot) => {
                self.macro_slot = slot.min(15);
                if let Some(ref cfg) = self.snap.config {
                    self.macro_steps = cfg.macros.get(self.macro_slot).cloned().unwrap_or_default();
                }
            }
            Message::MacroStepKindChanged(k) => self.macro_step_kind = k,
            Message::MacroStepValChanged(v) => self.macro_step_val = v,
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
            Message::RemoveMacroStep(idx) => {
                if idx < self.macro_steps.len() {
                    self.macro_steps.remove(idx);
                }
            }
            Message::SaveMacroSlot => {
                self.sampler.send(DeviceCommand::SetMacroSlot {
                    index: self.macro_slot,
                    steps: self.macro_steps.clone(),
                });
            }
            Message::ClearMacroSlot => {
                self.macro_steps.clear();
                self.sampler.send(DeviceCommand::SetMacroSlot {
                    index: self.macro_slot,
                    steps: Vec::new(),
                });
            }

            // Settings — immediate gain dispatch!
            Message::CheckUpdates => {
                let slot = Arc::clone(&self.pending_update);
                std::thread::spawn(move || {
                    let res = check_for_update();
                    if let Ok(mut g) = slot.lock() {
                        *g = Some(res);
                    }
                });
            }
            Message::InstallUpdate => {
                if let Some(Ok(ref info)) = self.update_info {
                    match download_and_apply_update(info) {
                        Ok(msg) => self.show_toast(msg, false),
                        Err(e) => {
                            let _ = open::that(&info.url);
                            self.show_toast(e, true);
                        }
                    }
                }
            }
            Message::GainRChanged(v) => {
                self.settings.led_gain[0] = (v * 100.0).round() / 100.0;
                self.sampler.send(DeviceCommand::SetGain(self.settings.led_gain));
            }
            Message::GainGChanged(v) => {
                self.settings.led_gain[1] = (v * 100.0).round() / 100.0;
                self.sampler.send(DeviceCommand::SetGain(self.settings.led_gain));
            }
            Message::GainBChanged(v) => {
                self.settings.led_gain[2] = (v * 100.0).round() / 100.0;
                self.sampler.send(DeviceCommand::SetGain(self.settings.led_gain));
            }
            Message::SetGainPreset(gain) => {
                self.settings.led_gain = gain;
                self.sampler.send(DeviceCommand::SetGain(gain));
                self.sampler.send(DeviceCommand::SetLight(self.light_draft));
                self.show_toast(
                    self.lang.t("Ganho RGB aplicado", "LED gain preset applied"),
                    false,
                );
            }
            Message::StartAtLoginToggled(v) => {
                self.settings.start_at_login = set_start_at_login(v);
                let _ = self.settings.save(&settings_path());
            }
            Message::CheckUpdatesPrefToggled(v) => {
                self.settings.check_updates = v;
                let _ = self.settings.save(&settings_path());
            }
        }
        Task::none()
    }

    fn send_feature_update(&mut self, mutate: impl FnOnce(&mut KeyboardFeature)) {
        if let Some(ref cfg) = self.snap.config {
            let mut f = cfg.feature;
            mutate(&mut f);
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
            .map_or(true, |e| e.has_color);
        if !supports_color {
            self.light_draft.effect = 2;
        }
        self.sampler.send(DeviceCommand::SetLight(self.light_draft));
    }

    pub fn accent(&self) -> Color {
        dynamic_accent([self.light_draft.r, self.light_draft.g, self.light_draft.b])
    }

    pub fn has_unsaved_changes(&self) -> bool {
        let Some(ref cfg) = self.snap.config else {
            return false;
        };
        if cfg.light != self.light_draft || cfg.box_light != self.box_draft {
            return true;
        }
        if (cfg.dead_band.top_mm - self.dead_top_mm).abs() > 0.01
            || (cfg.dead_band.bottom_mm - self.dead_bot_mm).abs() > 0.01
        {
            return true;
        }
        if let Some(&act) = cfg.actuation.first() {
            if (act - self.actuation_mm).abs() > 0.01 {
                return true;
            }
        }
        if let Some(rt) = cfg.rapid_trigger.first() {
            if rt.enabled != self.rt_enabled
                || (rt.press_mm - self.rt_press_mm).abs() > 0.01
                || (rt.release_mm - self.rt_release_mm).abs() > 0.01
            {
                return true;
            }
        }
        false
    }

    pub fn view(&self, _window: window::Id) -> Element<'_, Message> {
        self.view_shell()
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
