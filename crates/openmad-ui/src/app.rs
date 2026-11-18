//! Main OpenMAD-Rust iced application with Apple Dark Mode styling and PT-BR/EN support.

use crate::i18n::Lang;
use crate::theme::{
    badge_style, card_style, danger_button_style, elevated_card_style, input_style,
    nav_button_style, pick_list_style, primary_button_style, secondary_button_style,
    segmented_bar_style, sidebar_style, slider_style, swatch_button_style, tab_button_style,
    toast_style, warn_banner_style, ACCENT_BLUE, AMBER_WARN, BG_BASE, FONT_INTER, GREEN_OK,
    RED_DANGER, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::tray::{TrayAction, TrayHandle};
use crate::widgets::color_picker::{hsv_to_rgb, rgb_to_hsv, ColorPickerCanvas};
use crate::widgets::gauge::TravelGaugeBar;
use crate::widgets::keyboard::{BoardViewMode, KeyboardCanvas, KeyboardEvent};
use iced::widget::{
    button, column, container, pick_list, row, scrollable, slider, text, text_input, toggler,
    Column, Row, Space,
};
use iced::{time, window, Alignment, Color, Element, Length, Subscription, Task};
use openmad_core::device::FullConfig;
use openmad_core::features::{
    AdvancedKey, BoxLight, DeadBand, Dks, DksAction, KeyboardFeature, LightInfo, MacroStep,
    TapDance, LIGHT_EFFECTS,
};
use openmad_core::keycodes::{
    self, KEYBOARD_PICKER_CODES, LAYER_PICKER_CODES, SYSTEM_PICKER_CODES,
};
use openmad_core::paths::{onboard_path, profile_dir, settings_path, switcher_config_path};
use openmad_core::profile::{
    key_str, list_profiles, profile_path, write_onboard_name, Profile, RapidTriggerSpec,
    TriggerBinding,
};
use openmad_core::protocol::{AdvancedKeyMode, BRIGHTNESS_MAX, FlashOp, KNOWN_PROTOCOL_VERSION};
use openmad_core::sampler::{DeviceCommand, SamplerHandle, Snapshot};
use openmad_core::settings::{set_start_at_login, AppSettings};
use openmad_core::switcher::{
    list_windowed_processes, watcher_backend_name, Rule, SwitcherConfig, WindowedProcess,
};
use openmad_core::updater::{
    check_for_update, download_and_apply_update, UpdateInfo, APP_VERSION, REPO_SLUG,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const COLOR_PRESETS: &[(u8, u8, u8)] = &[
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
pub enum SidebarView {
    Equipment,
    Macros,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipTab {
    ChangeKey,
    Lighting,
    Performance,
    AdvancedKey,
    AppTrigger,
    OtherSettings,
}

impl EquipTab {
    const ALL: [Self; 6] = [
        Self::ChangeKey,
        Self::Lighting,
        Self::Performance,
        Self::AdvancedKey,
        Self::AppTrigger,
        Self::OtherSettings,
    ];

    fn title(self, lang: Lang) -> &'static str {
        match self {
            Self::ChangeKey => lang.t("Remapear Teclas", "Keymap"),
            Self::Lighting => lang.t("Iluminação", "Lighting"),
            Self::Performance => lang.t("Performance", "Performance"),
            Self::AdvancedKey => lang.t("Teclas Avançadas", "Advanced Key"),
            Self::AppTrigger => lang.t("Troca por App", "App Trigger"),
            Self::OtherSettings => lang.t("Sistema & Flash", "Other Settings"),
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
    SelectView(SidebarView),
    SelectEquipTab(EquipTab),
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

    // Settings Modal
    ToggleSettingsModal(bool),
    CheckUpdates,
    InstallUpdate,
    GainRChanged(f32),
    GainGChanged(f32),
    GainBChanged(f32),
    SetGainPreset([f32; 3]),
    StartAtLoginToggled(bool),
    CheckUpdatesPrefToggled(bool),
}

pub struct OpenMadApp {
    sampler: SamplerHandle,
    tray: Option<TrayHandle>,
    snap: Snapshot,
    window_id: Option<window::Id>,

    lang: Lang,
    view: SidebarView,
    equip_tab: EquipTab,
    perf_sub: PerfSubTab,
    pick_tab: KeyPickerTab,
    adv_sub: AdvSubTab,
    layer: usize,
    selected_keys: BTreeSet<usize>,

    profiles: Vec<String>,
    selected_profile: Option<String>,
    new_profile_name: String,

    light_draft: LightInfo,
    light_hue: f32,
    light_sat: f32,
    light_val: f32,
    light_hex: String,
    box_draft: BoxLight,

    actuation_mm: f32,
    rt_enabled: bool,
    rt_press_mm: f32,
    rt_release_mm: f32,
    dead_top_mm: f32,
    dead_bot_mm: f32,

    adv_slot: usize,
    adv_k1_r: String,
    adv_k1_c: String,
    adv_k2_r: String,
    adv_k2_c: String,
    adv_apc: f32,
    adv_rt: bool,
    mt_tap: String,
    mt_hold: String,
    mt_timer: String,
    dks_draft: Dks,

    switcher_cfg: SwitcherConfig,
    trigger_exe: String,
    trigger_title: String,
    processes: Vec<WindowedProcess>,

    macro_slot: usize,
    macro_steps: Vec<MacroStep>,
    macro_step_kind: String,
    macro_step_val: String,

    factory_confirm: String,
    show_settings: bool,
    settings: AppSettings,
    update_info: Option<Result<UpdateInfo, String>>,
    pending_update: Arc<Mutex<Option<Result<UpdateInfo, String>>>>,
    toast: Option<(String, bool, u8)>,
    synced_initial: bool,
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
        let (window_id, init_task) = if start_minimized {
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
            view: SidebarView::Equipment,
            equip_tab: EquipTab::Performance,
            perf_sub: PerfSubTab::Travel,
            pick_tab: KeyPickerTab::Keyboard,
            adv_sub: AdvSubTab::RappySnappy,
            layer: 0,
            selected_keys: BTreeSet::new(),
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
            show_settings: false,
            settings,
            update_info: None,
            pending_update,
            toast: None,
            synced_initial: false,
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
            Message::SelectView(v) => {
                self.show_settings = false;
                self.view = v;
            }
            Message::SelectEquipTab(t) => {
                self.equip_tab = t;
                if t == EquipTab::AppTrigger && self.processes.is_empty() {
                    self.processes = list_windowed_processes();
                }
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
            Message::CommitToFlash => self.sampler.send(DeviceCommand::CommitToFlash),
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

            // Settings Modal — immediate gain dispatch!
            Message::ToggleSettingsModal(v) => self.show_settings = v,
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

    pub fn view(&self, _window: window::Id) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();
        let main_area = if self.show_settings {
            self.view_settings()
        } else {
            self.view_main()
        };

        let root = row![sidebar, main_area]
            .width(Length::Fill)
            .height(Length::Fill);

        container(root)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(BG_BASE)),
                ..Default::default()
            })
            .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let l = self.lang;
        let header = row![
            column![
                text("OpenMAD").size(20).font(FONT_INTER).color(TEXT_PRIMARY),
                text(format!("Rust Edition · v{APP_VERSION}"))
                    .size(11)
                    .color(TEXT_MUTED),
            ]
            .spacing(2),
            Space::with_width(Length::Fill),
            button(text(if l.is_pt() { "PT" } else { "EN" }).size(11))
                .padding([4, 9])
                .style(secondary_button_style)
                .on_press(Message::ToggleLanguage),
        ]
        .align_y(Alignment::Center);

        let nav = column![
            button(text(l.t("Equipamento", "Equipment")).size(13))
                .width(Length::Fill)
                .padding([9, 12])
                .style(nav_button_style(
                    self.view == SidebarView::Equipment && !self.show_settings
                ))
                .on_press(Message::SelectView(SidebarView::Equipment)),
            button(text("Macros").size(13))
                .width(Length::Fill)
                .padding([9, 12])
                .style(nav_button_style(
                    self.view == SidebarView::Macros && !self.show_settings
                ))
                .on_press(Message::SelectView(SidebarView::Macros)),
            button(text(l.t("Firmware ↗", "Firmware ↗")).size(13))
                .width(Length::Fill)
                .padding([9, 12])
                .style(nav_button_style(false))
                .on_press(Message::OpenFirmwareUrl),
        ]
        .spacing(4);

        let prof_controls = row![
            text(l.t("PERFIS", "PROFILES")).size(10).color(TEXT_MUTED),
            Space::with_width(Length::Fill),
            button(text(l.t("Importar", "Import")).size(11))
                .padding([3, 7])
                .style(secondary_button_style)
                .on_press(Message::ImportProfile),
            button(text("Flash").size(11))
                .padding([3, 7])
                .style(secondary_button_style)
                .on_press(Message::SaveProfileOnboard),
        ]
        .align_y(Alignment::Center);

        let new_prof_row = row![
            text_input(l.t("Nome do perfil…", "Profile name…"), &self.new_profile_name)
                .on_input(Message::NewProfileNameChanged)
                .padding([6, 9])
                .size(12)
                .style(input_style),
            button(text("+").size(13))
                .padding([5, 11])
                .style(primary_button_style)
                .on_press(Message::CreateProfile(false)),
        ]
        .spacing(6);

        let mut plist = Column::new().spacing(3);
        for name in &self.profiles {
            let active = self.selected_profile.as_deref() == Some(name.as_str());
            let is_onboard = self.snap.onboard_profile.as_deref() == Some(name.as_str());
            let label = if is_onboard {
                format!("{name}  ●")
            } else {
                name.clone()
            };
            plist = plist.push(
                button(text(label).size(12))
                    .width(Length::Fill)
                    .padding([7, 10])
                    .style(nav_button_style(active))
                    .on_press(Message::SelectProfile(name.clone())),
            );
        }

        let prof_actions = column![
            button(
                text(
                    self.selected_profile
                        .as_deref()
                        .map(|n| format!("{} ({n})", l.t("Salvar no Perfil", "Save to Profile")))
                        .unwrap_or_else(|| l.t("Salvar no Perfil", "Save to Profile").to_string())
                )
                .size(11)
            )
            .width(Length::Fill)
            .padding([6, 10])
            .style(primary_button_style)
            .on_press(Message::SaveCurrentProfile),
            row![
                button(text(l.t("Exp", "Exp")).size(11))
                    .padding([4, 8])
                    .style(secondary_button_style)
                    .on_press(Message::ExportProfile),
                button(text("Dup").size(11))
                    .padding([4, 8])
                    .style(secondary_button_style)
                    .on_press(Message::DuplicateProfile),
                button(text(l.t("Ren", "Ren")).size(11))
                    .padding([4, 8])
                    .style(secondary_button_style)
                    .on_press(Message::RenameProfile),
                button(text(l.t("Del", "Del")).size(11))
                    .padding([4, 8])
                    .style(danger_button_style)
                    .on_press(Message::DeleteProfile),
            ]
            .spacing(4),
        ]
        .spacing(6);

        let status_color = if self.snap.connected {
            if self.snap.read_only {
                AMBER_WARN
            } else {
                GREEN_OK
            }
        } else {
            RED_DANGER
        };
        let status_label = if self.snap.connected {
            if self.snap.read_only {
                format!("● {} ({})", self.snap.board.name, l.t("Leitura", "Read-Only"))
            } else {
                format!("● {}", self.snap.board.name)
            }
        } else if self.snap.in_bootloader {
            l.t("● Modo Bootloader", "● Bootloader Mode").to_string()
        } else {
            l.t("● Desconectado", "● Disconnected").to_string()
        };

        let status_badge = container(text(status_label).size(11).color(status_color))
            .width(Length::Fill)
            .padding([6, 10])
            .style(badge_style(status_color));

        let footer = column![
            status_badge,
            row![
                button(text(l.t("⚙  Configurações", "⚙  Settings")).size(12))
                    .width(Length::Fill)
                    .padding([8, 10])
                    .style(nav_button_style(self.show_settings))
                    .on_press(Message::ToggleSettingsModal(!self.show_settings)),
                button(text(l.t("Bandeja", "Tray")).size(12))
                    .padding([8, 10])
                    .style(secondary_button_style)
                    .on_press(Message::HideToTray),
            ]
            .spacing(6),
        ]
        .spacing(8);

        let content = column![
            header,
            nav,
            prof_controls,
            new_prof_row,
            scrollable(plist).height(Length::Fill),
            prof_actions,
            footer,
        ]
        .spacing(12)
        .padding(16);

        container(content)
            .width(Length::Fixed(252.0))
            .height(Length::Fill)
            .style(sidebar_style)
            .into()
    }

    fn view_main(&self) -> Element<'_, Message> {
        let l = self.lang;
        let mut col = Column::new().spacing(14).padding(20);

        if !self.snap.connected {
            if let Some(ref err) = self.snap.error {
                col = col.push(
                    container(text(err).size(12).color(AMBER_WARN))
                        .width(Length::Fill)
                        .padding([8, 12])
                        .style(warn_banner_style),
                );
            }
        }
        if self.snap.connected && !self.snap.firmware_verified {
            let pv = self.snap.protocol_version.unwrap_or(0);
            col = col.push(
                container(
                    text(format!(
                        "{} v{pv} != v{KNOWN_PROTOCOL_VERSION}. {}",
                        l.t("Aviso: Protocolo do firmware", "Warning: Firmware protocol"),
                        l.t(
                            "Faça um backup antes de gravar.",
                            "Take a backup before writing."
                        )
                    ))
                    .size(12)
                    .color(AMBER_WARN),
                )
                .width(Length::Fill)
                .padding([8, 12])
                .style(warn_banner_style),
            );
        }
        if self.snap.connected && self.snap.read_only {
            col = col.push(
                container(
                    row![
                        text(
                            self.snap
                                .read_only_reason
                                .as_deref()
                                .unwrap_or(l.t("Placa em modo somente leitura", "Board opened read-only"))
                        )
                        .size(12)
                        .color(AMBER_WARN),
                        Space::with_width(Length::Fill),
                        button(text(l.t("Desbloquear Escrita", "Unlock Writes")).size(12))
                            .padding([4, 10])
                            .style(secondary_button_style)
                            .on_press(Message::UnlockBoard),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([8, 12])
                .style(warn_banner_style),
            );
        }

        if let Some((ref msg, is_err, _)) = self.toast {
            col = col.push(
                container(
                    row![
                        text(msg)
                            .size(12)
                            .color(if is_err { RED_DANGER } else { GREEN_OK }),
                        Space::with_width(Length::Fill),
                        button(text("✕").size(11))
                            .padding([2, 6])
                            .style(secondary_button_style)
                            .on_press(Message::DismissToast),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([8, 12])
                .style(toast_style(is_err)),
            );
        }

        match self.view {
            SidebarView::Equipment => {
                let mut tabs = Row::new().spacing(4);
                for t in EquipTab::ALL {
                    tabs = tabs.push(
                        button(text(t.title(l)).size(12))
                            .padding([7, 13])
                            .style(tab_button_style(self.equip_tab == t))
                            .on_press(Message::SelectEquipTab(t)),
                    );
                }
                let mut bar_row = Row::new().spacing(10).align_y(Alignment::Center);
                bar_row = bar_row.push(
                    container(tabs)
                        .padding(4)
                        .style(segmented_bar_style),
                );
                if self.selected_profile.is_some() {
                    bar_row = bar_row.push(Space::with_width(Length::Fill));
                    bar_row = bar_row.push(
                        button(text(l.t("Salvar no Perfil", "Save to Profile")).size(12))
                            .padding([7, 13])
                            .style(primary_button_style)
                            .on_press(Message::SaveCurrentProfile),
                    );
                }
                col = col.push(bar_row);

                if matches!(
                    self.equip_tab,
                    EquipTab::ChangeKey
                        | EquipTab::Lighting
                        | EquipTab::Performance
                        | EquipTab::AdvancedKey
                ) {
                    let mode = match self.equip_tab {
                        EquipTab::Lighting => BoardViewMode::Lighting,
                        EquipTab::Performance => BoardViewMode::Performance,
                        EquipTab::AdvancedKey => BoardViewMode::Advanced,
                        _ => BoardViewMode::Keymap,
                    };
                    let cfg_ref = self.snap.config.as_ref();
                    let kb = KeyboardCanvas {
                        rows: self.snap.board.rows,
                        cols: self.snap.board.cols,
                        populated: &self.snap.populated,
                        selected: &self.selected_keys,
                        mode,
                        layer: self.layer,
                        keymap: cfg_ref.map(|c| c.keymap.as_slice()).unwrap_or(&[]),
                        actuation: cfg_ref.map(|c| c.actuation.as_slice()).unwrap_or(&[]),
                        trip_mm: &self.snap.trip_mm,
                        key_colors: cfg_ref.map(|c| c.key_colors.as_slice()).unwrap_or(&[]),
                        light_effect: self.light_draft.effect,
                        light_color: [self.light_draft.r, self.light_draft.g, self.light_draft.b],
                    };
                    col = col.push(kb.view(Message::Keyboard));

                    let sel_hint = if self.selected_keys.is_empty() {
                        l.t(
                            "Nenhuma tecla selecionada (ajustes aplicam em todo o teclado)",
                            "0 keys selected (edits apply to the whole keyboard)",
                        )
                        .to_string()
                    } else {
                        format!(
                            "{} {}",
                            self.selected_keys.len(),
                            l.t("tecla(s) selecionada(s)", "key(s) selected")
                        )
                    };

                    let sel_bar = row![
                        text(sel_hint).size(12).color(TEXT_SECONDARY),
                        Space::with_width(Length::Fill),
                        button(text(l.t("Selecionar Todas", "Select All")).size(11))
                            .padding([4, 10])
                            .style(secondary_button_style)
                            .on_press(Message::SelectAllKeys),
                        button(text(l.t("Limpar", "Clear")).size(11))
                            .padding([4, 10])
                            .style(secondary_button_style)
                            .on_press(Message::SelectNoKeys),
                        button(text(l.t("Inverter", "Invert")).size(11))
                            .padding([4, 10])
                            .style(secondary_button_style)
                            .on_press(Message::InvertKeys),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center);
                    col = col.push(sel_bar);
                }

                let tab_body = match self.equip_tab {
                    EquipTab::ChangeKey => self.view_change_key(),
                    EquipTab::Lighting => self.view_lighting(),
                    EquipTab::Performance => self.view_performance(),
                    EquipTab::AdvancedKey => self.view_advanced_key(),
                    EquipTab::AppTrigger => self.view_app_trigger(),
                    EquipTab::OtherSettings => self.view_other_settings(),
                };
                col = col.push(scrollable(tab_body).height(Length::Fill));
            }
            SidebarView::Macros => {
                col = col.push(scrollable(self.view_macros()).height(Length::Fill));
            }
        }

        col.width(Length::Fill).height(Length::Fill).into()
    }

    fn view_change_key(&self) -> Element<'_, Message> {
        let l = self.lang;
        let mut layer_row = Row::new().spacing(6).align_y(Alignment::Center);
        layer_row = layer_row.push(text(l.t("Camada:", "Layer:")).size(12).color(TEXT_SECONDARY));
        for idx in 0..4 {
            layer_row = layer_row.push(
                button(text(format!("L{idx}")).size(12))
                    .padding([5, 12])
                    .style(tab_button_style(self.layer == idx))
                    .on_press(Message::SelectLayer(idx)),
            );
        }
        layer_row = layer_row.push(Space::with_width(Length::Fill));
        layer_row = layer_row.push(
            button(text(l.t("Aplicar Keymap Completo (EEPROM)", "Apply Full Keymap (EEPROM)")).size(12))
                .padding([6, 12])
                .style(primary_button_style)
                .on_press(Message::ApplyFullKeymap),
        );

        let pick_tabs = row![
            button(text(l.t("Teclas Padrão", "Keyboard Key")).size(12))
                .padding([5, 10])
                .style(tab_button_style(self.pick_tab == KeyPickerTab::Keyboard))
                .on_press(Message::SelectKeyPickerTab(KeyPickerTab::Keyboard)),
            button(text(l.t("Sistema / Mídia", "System / Media")).size(12))
                .padding([5, 10])
                .style(tab_button_style(self.pick_tab == KeyPickerTab::System))
                .on_press(Message::SelectKeyPickerTab(KeyPickerTab::System)),
            button(text("Macros").size(12))
                .padding([5, 10])
                .style(tab_button_style(self.pick_tab == KeyPickerTab::Macro))
                .on_press(Message::SelectKeyPickerTab(KeyPickerTab::Macro)),
            button(text(l.t("Camadas (Fn)", "Layer / Special")).size(12))
                .padding([5, 10])
                .style(tab_button_style(self.pick_tab == KeyPickerTab::Layer))
                .on_press(Message::SelectKeyPickerTab(KeyPickerTab::Layer)),
        ]
        .spacing(6);

        let codes: Vec<(u16, String)> = match self.pick_tab {
            KeyPickerTab::Keyboard => KEYBOARD_PICKER_CODES
                .iter()
                .map(|&(k, lbl)| (k, lbl.to_string()))
                .collect(),
            KeyPickerTab::System => SYSTEM_PICKER_CODES
                .iter()
                .map(|&(k, lbl)| (k, lbl.to_string()))
                .collect(),
            KeyPickerTab::Macro => (0..16u16).map(|i| (0x7700 + i, format!("M{i}"))).collect(),
            KeyPickerTab::Layer => LAYER_PICKER_CODES
                .iter()
                .map(|&(k, lbl)| (k, lbl.to_string()))
                .collect(),
        };

        let mut grid = Column::new().spacing(6);
        for chunk in codes.chunks(10) {
            let mut r = Row::new().spacing(6);
            for (kc, label) in chunk {
                r = r.push(
                    button(text(label.clone()).size(11))
                        .padding([6, 10])
                        .style(secondary_button_style)
                        .on_press(Message::AssignKeycode(*kc)),
                );
            }
            grid = grid.push(r);
        }

        container(
            column![
                layer_row,
                text(l.t(
                    "Clique em uma tecla do teclado acima e escolha a nova função abaixo. Alterações de keymap são gravadas diretamente na EEPROM.",
                    "Click a key on the keyboard above, then pick its new keycode below (writes to EEPROM)."
                ))
                .size(12)
                .color(TEXT_MUTED),
                pick_tabs,
                grid,
            ]
            .spacing(12)
            .padding(18),
        )
        .width(Length::Fill)
        .style(card_style)
        .into()
    }

    fn view_lighting(&self) -> Element<'_, Message> {
        let l = self.lang;
        let pt = l.is_pt();
        let effect_names: Vec<String> = LIGHT_EFFECTS
            .iter()
            .map(|e| format!("{:02} · {}", e.id, e.label(pt)))
            .collect();
        let cur_meta = LIGHT_EFFECTS.iter().find(|e| e.id == self.light_draft.effect);
        let current_eff = cur_meta.map(|e| format!("{:02} · {}", e.id, e.label(pt)));
        let has_color = cur_meta.map_or(true, |e| e.has_color);
        let has_speed = cur_meta.map_or(true, |e| e.has_speed);

        let eff_picker = pick_list(effect_names, current_eff, |sel| {
            let id = sel
                .split('·')
                .next()
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(13);
            Message::LightEffectSelected(id)
        })
        .style(pick_list_style)
        .padding([6, 10]);

        let swatch = container(Space::with_width(Length::Fixed(34.0)).height(Length::Fixed(34.0)))
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(
                    self.light_draft.r,
                    self.light_draft.g,
                    self.light_draft.b,
                ))),
                border: iced::Border {
                    color: TEXT_SECONDARY,
                    width: 1.2,
                    radius: 8.0.into(),
                },
                ..Default::default()
            });

        let mut presets = Row::new().spacing(8).align_y(Alignment::Center);
        for &(r, g, b) in COLOR_PRESETS {
            presets = presets.push(
                button(Space::with_width(Length::Fixed(20.0)).height(Length::Fixed(20.0)))
                    .padding(0)
                    .style(swatch_button_style(Color::from_rgb8(r, g, b)))
                    .on_press(Message::LightColorPreset(r, g, b)),
            );
        }

        let effect_hint = if !has_color {
            l.t(
                "Efeito com cores automáticas — ao clicar em uma cor abaixo, muda automaticamente para Cor Fixa.",
                "Auto-spectrum effect — clicking any color below automatically switches to Constant Color.",
            )
        } else if self.light_draft.effect == 1 {
            l.t(
                "Modo Personalizado ativo: clicar em qualquer cor pinta imediatamente as teclas selecionadas.",
                "Customization mode active: clicking any color immediately paints the selected keys.",
            )
        } else {
            l.t(
                "Clique ou arraste no seletor de cor para mudar a cor do teclado ao vivo.",
                "Click or drag in the color picker to change keyboard color live.",
            )
        };

        let hsv_picker = ColorPickerCanvas {
            hue: self.light_hue,
            sat: self.light_sat,
            val: self.light_val,
        }
        .view(Message::LightHsvChanged);

        let color_controls = column![
            row![
                swatch,
                column![
                    text(l.t("Código HEX", "HEX Code")).size(10).color(TEXT_MUTED),
                    text_input("#0A84FF", &self.light_hex)
                        .on_input(Message::LightHexChanged)
                        .width(Length::Fixed(104.0))
                        .padding([5, 8])
                        .size(12)
                        .style(input_style),
                ]
                .spacing(2),
                Space::with_width(Length::Fixed(8.0)),
                column![
                    text(l.t("Cores rápidas", "Presets")).size(10).color(TEXT_MUTED),
                    presets,
                ]
                .spacing(4),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            slider_row(
                format!("{} ({})", l.t("Vermelho", "Red"), self.light_draft.r),
                0.0..=255.0,
                self.light_draft.r as f32,
                1.0,
                |v| Message::LightRChanged(v as u8),
            ),
            slider_row(
                format!("{} ({})", l.t("Verde", "Green"), self.light_draft.g),
                0.0..=255.0,
                self.light_draft.g as f32,
                1.0,
                |v| Message::LightGChanged(v as u8),
            ),
            slider_row(
                format!("{} ({})", l.t("Azul", "Blue"), self.light_draft.b),
                0.0..=255.0,
                self.light_draft.b as f32,
                1.0,
                |v| Message::LightBChanged(v as u8),
            ),
        ]
        .spacing(10)
        .width(Length::Fill);

        let mut main_col = column![
            row![
                column![
                    text(l.t("Iluminação Global", "Global Lighting")).size(15).color(TEXT_PRIMARY),
                    text(effect_hint).size(12).color(TEXT_MUTED),
                ]
                .spacing(3),
                Space::with_width(Length::Fill),
                button(text(l.t("Pintar Teclas", "Paint Keys")).size(12))
                    .padding([6, 12])
                    .style(secondary_button_style)
                    .on_press(Message::PaintSelectedKeys),
                button(text(l.t("Salvar na Flash", "Save to Flash")).size(12))
                    .padding([6, 12])
                    .style(primary_button_style)
                    .on_press(Message::SaveLighting),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text(l.t("Efeito:", "Effect:")).size(12).color(TEXT_SECONDARY),
                eff_picker
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            slider_row(
                format!(
                    "{} ({}/{BRIGHTNESS_MAX})",
                    l.t("Brilho", "Brightness"),
                    self.light_draft.brightness
                ),
                0.0..=(BRIGHTNESS_MAX as f32),
                self.light_draft.brightness as f32,
                1.0,
                |v| Message::LightBrightnessChanged(v as u8),
            ),
        ]
        .spacing(11)
        .padding(18);

        if has_speed {
            main_col = main_col.push(slider_row(
                format!("{} ({})", l.t("Velocidade", "Speed"), self.light_draft.speed),
                1.0..=255.0,
                self.light_draft.speed.max(1) as f32,
                1.0,
                |v| Message::LightSpeedChanged(v as u8),
            ));
        }

        main_col = main_col.push(
            row![hsv_picker, color_controls]
                .spacing(18)
                .align_y(Alignment::Center),
        );

        let main_card = container(main_col).width(Length::Fill).style(card_style);

        let box_card = container(
            column![
                column![
                    text(l.t("Iluminação Lateral / Caixa", "Box / Side Lighting"))
                        .size(15)
                        .color(TEXT_PRIMARY),
                    text(l.t(
                        "Controla os LEDs secundários/laterais em modelos com faixa externa.",
                        "Controls secondary side/box LEDs on models equipped with side lighting."
                    ))
                    .size(12)
                    .color(TEXT_MUTED),
                ]
                .spacing(3),
                slider_row(
                    format!("{} ({})", l.t("Modo", "Mode"), self.box_draft.mode),
                    0.0..=10.0,
                    self.box_draft.mode as f32,
                    1.0,
                    |v| Message::BoxModeChanged(v as u8),
                ),
                slider_row(
                    format!("{} ({})", l.t("Colorido", "Colorful"), self.box_draft.colorful),
                    0.0..=1.0,
                    self.box_draft.colorful as f32,
                    1.0,
                    |v| Message::BoxColorfulChanged(v as u8),
                ),
                slider_row(
                    format!("{} ({})", l.t("Brilho", "Brightness"), self.box_draft.brightness),
                    0.0..=255.0,
                    self.box_draft.brightness as f32,
                    1.0,
                    |v| Message::BoxBrightnessChanged(v as u8),
                ),
                slider_row(
                    format!("{} ({})", l.t("Velocidade", "Speed"), self.box_draft.speed),
                    0.0..=255.0,
                    self.box_draft.speed as f32,
                    1.0,
                    |v| Message::BoxSpeedChanged(v as u8),
                ),
            ]
            .spacing(10)
            .padding(18),
        )
        .width(Length::Fill)
        .style(card_style);

        column![main_card, box_card].spacing(12).into()
    }

    fn view_performance(&self) -> Element<'_, Message> {
        let l = self.lang;
        let sub_tabs = container(
            row![
                button(text(l.t("Curso & Rapid Trigger", "Travel & Rapid Trigger")).size(12))
                    .padding([6, 12])
                    .style(tab_button_style(self.perf_sub == PerfSubTab::Travel))
                    .on_press(Message::SelectPerfSubTab(PerfSubTab::Travel)),
                button(text(l.t("Calibração", "Calibration")).size(12))
                    .padding([6, 12])
                    .style(tab_button_style(self.perf_sub == PerfSubTab::Calibration))
                    .on_press(Message::SelectPerfSubTab(PerfSubTab::Calibration)),
                button(text(l.t("Modos & Recursos", "Features & Modes")).size(12))
                    .padding([6, 12])
                    .style(tab_button_style(self.perf_sub == PerfSubTab::Performance))
                    .on_press(Message::SelectPerfSubTab(PerfSubTab::Performance)),
            ]
            .spacing(6),
        )
        .padding(4)
        .style(segmented_bar_style);

        let body: Element<'_, Message> = match self.perf_sub {
            PerfSubTab::Travel => {
                // Auto-focus whichever key is currently pressed deepest; fallback to first selected key
                let deepest = self
                    .snap
                    .trip_mm
                    .iter()
                    .enumerate()
                    .filter(|(i, &mm)| mm > 0.04 && self.snap.populated.get(*i).copied().unwrap_or(true))
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(i, _)| i);
                let focus_idx = deepest
                    .or_else(|| self.selected_keys.iter().next().copied())
                    .unwrap_or(0);

                let live_mm = self.snap.trip_mm.get(focus_idx).copied().unwrap_or(0.0);
                let peak_mm = self.snap.peak_mm.get(focus_idx).copied().unwrap_or(0.0);
                let focus_name = self
                    .snap
                    .config
                    .as_ref()
                    .and_then(|c| {
                        let cols = self.snap.board.cols;
                        let r = focus_idx / cols;
                        let c_col = focus_idx % cols;
                        let decoded = keycodes::decode_keymap(&c.keymap, self.layer, self.snap.board.rows, cols);
                        decoded.get(r).and_then(|row| row.get(c_col)).map(|&kc| keycodes::label(kc))
                    })
                    .unwrap_or_else(|| format!("#{focus_idx}"));

                let gauge_card = container(
                    column![
                        row![
                            column![
                                text(format!(
                                    "{} · {}",
                                    l.t("MEDIDOR DE CURSO AO VIVO", "LIVE TRAVEL GAUGE"),
                                    focus_name
                                ))
                                .size(10)
                                .color(TEXT_MUTED),
                                row![
                                    text(format!("{live_mm:.2} mm"))
                                        .size(26)
                                        .color(if live_mm >= self.actuation_mm {
                                            GREEN_OK
                                        } else {
                                            TEXT_PRIMARY
                                        }),
                                    Space::with_width(Length::Fixed(18.0)),
                                    column![
                                        text(l.t("PICO", "PEAK")).size(10).color(TEXT_MUTED),
                                        text(format!("{peak_mm:.2} mm")).size(16).color(ACCENT_BLUE),
                                    ],
                                ]
                                .align_y(Alignment::Center),
                            ]
                            .spacing(4),
                            Space::with_width(Length::Fill),
                            button(text(l.t("Zerar Picos", "Reset Peaks")).size(12))
                                .padding([6, 12])
                                .style(secondary_button_style)
                                .on_press(Message::ResetPeaks),
                        ]
                        .align_y(Alignment::Center),
                        TravelGaugeBar {
                            live_mm,
                            peak_mm,
                            actuation_mm: self.actuation_mm,
                        }
                        .view(),
                        row![
                            text("0.00 mm").size(10).color(TEXT_MUTED),
                            Space::with_width(Length::Fill),
                            text(format!(
                                "{}: {:.2} mm",
                                l.t("Ponto de atuação", "Actuation"),
                                self.actuation_mm
                            ))
                            .size(10)
                            .color(TEXT_SECONDARY),
                            Space::with_width(Length::Fill),
                            text("3.50 mm").size(10).color(TEXT_MUTED),
                        ],
                    ]
                    .spacing(10)
                    .padding(18),
                )
                .width(Length::Fill)
                .style(card_style);

                let act_card = container(
                    column![
                        row![
                            column![
                                text(l.t("Ponto de Atuação", "Actuation Point"))
                                    .size(15)
                                    .color(TEXT_PRIMARY),
                                text(l.t(
                                    "Distância que a tecla precisa descer para registrar o clique (0.10 mm = ultra sensível, 2.00 mm = padrão).",
                                    "Travel distance required to register a keystroke (0.10 mm = ultra-sensitive, 2.00 mm = standard)."
                                ))
                                .size(12)
                                .color(TEXT_MUTED),
                            ]
                            .spacing(3),
                            Space::with_width(Length::Fill),
                            button(text(l.t("Aplicar na Seleção", "Apply to Selection")).size(12))
                                .padding([6, 12])
                                .style(secondary_button_style)
                                .on_press(Message::ApplyActuation),
                        ]
                        .align_y(Alignment::Center),
                        slider_row(
                            format!("{} ({:.2} mm)", l.t("Atuação", "Actuation"), self.actuation_mm),
                            0.10..=3.50,
                            self.actuation_mm.min(3.50),
                            0.05,
                            Message::ActuationChanged,
                        ),
                    ]
                    .spacing(10)
                    .padding(18),
                )
                .width(Length::Fill)
                .style(card_style);

                let rt_card = container(
                    column![
                        row![
                            column![
                                row![
                                    text("Rapid Trigger").size(15).color(TEXT_PRIMARY),
                                    Space::with_width(Length::Fixed(12.0)),
                                    toggler(self.rt_enabled).on_toggle(Message::RtEnabledToggled),
                                ]
                                .align_y(Alignment::Center),
                                text(l.t(
                                    "Reinicia a tecla instantaneamente ao começar a soltar, permitindo repetições muito mais rápidas.",
                                    "Resets the key immediately as you begin releasing, enabling rapid repeated inputs."
                                ))
                                .size(12)
                                .color(TEXT_MUTED),
                            ]
                            .spacing(3),
                            Space::with_width(Length::Fill),
                            button(text(l.t("Aplicar na Seleção", "Apply to Selection")).size(12))
                                .padding([6, 12])
                                .style(secondary_button_style)
                                .on_press(Message::ApplyRapidTrigger),
                        ]
                        .align_y(Alignment::Center),
                        slider_row(
                            format!(
                                "{} ({:.2} mm)",
                                l.t("Sensibilidade Pressionar", "Press Sensitivity"),
                                self.rt_press_mm
                            ),
                            0.05..=2.00,
                            self.rt_press_mm,
                            0.05,
                            Message::RtPressChanged,
                        ),
                        slider_row(
                            format!(
                                "{} ({:.2} mm)",
                                l.t("Sensibilidade Soltar", "Release Sensitivity"),
                                self.rt_release_mm
                            ),
                            0.05..=2.00,
                            self.rt_release_mm,
                            0.05,
                            Message::RtReleaseChanged,
                        ),
                    ]
                    .spacing(10)
                    .padding(18),
                )
                .width(Length::Fill)
                .style(card_style);

                let db_card = container(
                    column![
                        column![
                            text(l.t("Zona Morta (Dead Band)", "Dead Band"))
                                .size(15)
                                .color(TEXT_PRIMARY),
                            text(l.t(
                                "Margem de proteção no repouso (topo) e no fim de curso (fundo) para evitar toques acidentais ou falhas no fundo.",
                                "Safety margin at rest (top) and bottom-out to avoid accidental triggers or chatter."
                            ))
                            .size(12)
                            .color(TEXT_MUTED),
                        ]
                        .spacing(3),
                        slider_row(
                            format!(
                                "{} ({:.2} mm)",
                                l.t("Zona Morta Topo", "Top Dead Zone"),
                                self.dead_top_mm
                            ),
                            0.00..=1.00,
                            self.dead_top_mm,
                            0.05,
                            Message::DeadBandTopChanged,
                        ),
                        slider_row(
                            format!(
                                "{} ({:.2} mm)",
                                l.t("Zona Morta Fundo", "Bottom Dead Zone"),
                                self.dead_bot_mm
                            ),
                            0.00..=1.00,
                            self.dead_bot_mm,
                            0.05,
                            Message::DeadBandBottomChanged,
                        ),
                    ]
                    .spacing(10)
                    .padding(18),
                )
                .width(Length::Fill)
                .style(card_style);

                column![gauge_card, act_card, rt_card, db_card]
                    .spacing(12)
                    .into()
            }
            PerfSubTab::Calibration => {
                let calibrated = self.snap.populated.iter().filter(|&&p| p).count();
                container(
                    column![
                        text(l.t("Calibração dos Switches", "Switch Calibration"))
                            .size(15)
                            .color(TEXT_PRIMARY),
                        text(format!(
                            "{}: {calibrated} / {}",
                            l.t("Switches calibrados detectados", "Calibrated switches detected"),
                            self.snap.board.total_keys()
                        ))
                        .size(12)
                        .color(TEXT_SECONDARY),
                        text(l.t(
                            "Clique em \"Iniciar Calibração\", pressione todas as teclas firmemente até o final e clique em \"Finalizar Calibração\".",
                            "Click Start Calibration, press every key fully to bottom-out, then click Finish Calibration."
                        ))
                        .size(12)
                        .color(TEXT_MUTED),
                        row![
                            button(text(l.t("Iniciar Calibração", "Start Calibration")).size(12))
                                .padding([7, 14])
                                .style(primary_button_style)
                                .on_press(Message::StartCalibration),
                            button(text(l.t("Finalizar Calibração", "Finish Calibration")).size(12))
                                .padding([7, 14])
                                .style(secondary_button_style)
                                .on_press(Message::FinishCalibration),
                        ]
                        .spacing(10),
                    ]
                    .spacing(10)
                    .padding(18),
                )
                .width(Length::Fill)
                .style(card_style)
                .into()
            }
            PerfSubTab::Performance => {
                let cfg = self.snap.config.as_ref();
                let gm = cfg.map(|c| c.game_mode != 0).unwrap_or(false);
                let bo = cfg.map(|c| c.bottom_optimize != 0).unwrap_or(false);
                let f = cfg.map(|c| c.feature).unwrap_or_default();

                container(
                    column![
                        text(l.t("Modos e Recursos de Hardware", "Hardware Modes & Flags"))
                            .size(15)
                            .color(TEXT_PRIMARY),
                        toggle_desc_row(
                            l.t("Modo Jogo (Game Mode)", "Game Mode"),
                            l.t("Prioriza resposta mínima de latência no controlador.", "Optimizes controller scan loop for minimum gaming latency."),
                            gm,
                            Message::GameModeToggled,
                        ),
                        toggle_desc_row(
                            l.t("Otimização de Fim de Curso", "Bottom Optimize"),
                            l.t("Suaviza a leitura do sensor quando o switch chega ao fundo.", "Stabilizes Hall sensor reading at bottom-out."),
                            bo,
                            Message::BottomOptimizeToggled,
                        ),
                        toggle_desc_row(
                            l.t("Área RGB", "RGB Area"),
                            l.t("Habilita iluminação de área secundária no teclado.", "Enables secondary RGB lighting zone."),
                            f.rgb_area != 0,
                            Message::FeatureRgbAreaToggled,
                        ),
                        toggle_desc_row(
                            l.t("Inverter WASD / Setas", "Swap WASD / Arrows"),
                            l.t("Troca a função física das teclas WASD pelas setas direcionais.", "Swaps WASD keys with arrow keys."),
                            f.wasd_switch != 0,
                            Message::FeatureWasdToggled,
                        ),
                        toggle_desc_row(
                            l.t("Modo macOS", "macOS Mode"),
                            l.t("Ajusta modificadores (Alt/Cmd) para o padrão Mac.", "Swaps Option/Command modifiers for macOS."),
                            f.mac_switch != 0,
                            Message::FeatureMacToggled,
                        ),
                        toggle_desc_row(
                            l.t("Trava Tecla Windows (Win Lock)", "Win Lock"),
                            l.t("Desabilita a tecla Super/Windows para evitar acidentes em jogos.", "Disables the Windows/Super key to prevent accidental presses."),
                            f.win_lock != 0,
                            Message::FeatureWinLockToggled,
                        ),
                        toggle_desc_row(
                            "NKRO (N-Key Rollover)",
                            l.t("Permite registrar todas as teclas simultaneamente sem ghosting.", "Allows simultaneous registration of all keys without ghosting."),
                            f.nkro_switch != 0,
                            Message::FeatureNkroToggled,
                        ),
                    ]
                    .spacing(12)
                    .padding(18),
                )
                .width(Length::Fill)
                .style(card_style)
                .into()
            }
        };

        column![sub_tabs, body].spacing(12).into()
    }

    fn view_advanced_key(&self) -> Element<'_, Message> {
        let l = self.lang;
        let sub_tabs = container(
            row![
                button(text("Rappy Snappy (RS)").size(12))
                    .padding([6, 12])
                    .style(tab_button_style(self.adv_sub == AdvSubTab::RappySnappy))
                    .on_press(Message::SelectAdvSubTab(AdvSubTab::RappySnappy)),
                button(text("SOCD").size(12))
                    .padding([6, 12])
                    .style(tab_button_style(self.adv_sub == AdvSubTab::Socd))
                    .on_press(Message::SelectAdvSubTab(AdvSubTab::Socd)),
                button(text("Hold / Click (MT)").size(12))
                    .padding([6, 12])
                    .style(tab_button_style(self.adv_sub == AdvSubTab::HoldClick))
                    .on_press(Message::SelectAdvSubTab(AdvSubTab::HoldClick)),
                button(text("Dynamic Key Stroke (DKS)").size(12))
                    .padding([6, 12])
                    .style(tab_button_style(self.adv_sub == AdvSubTab::Dks))
                    .on_press(Message::SelectAdvSubTab(AdvSubTab::Dks)),
                button(text("Toggle (TGL)").size(12))
                    .padding([6, 12])
                    .style(tab_button_style(self.adv_sub == AdvSubTab::Toggle))
                    .on_press(Message::SelectAdvSubTab(AdvSubTab::Toggle)),
            ]
            .spacing(6),
        )
        .padding(4)
        .style(segmented_bar_style);

        let mut slot_row = Row::new().spacing(4).align_y(Alignment::Center);
        slot_row = slot_row.push(text("Slot:").size(12).color(TEXT_SECONDARY));
        for i in 0..16 {
            slot_row = slot_row.push(
                button(text(format!("{i}")).size(11))
                    .padding([4, 8])
                    .style(tab_button_style(self.adv_slot == i))
                    .on_press(Message::AdvSlotSelected(i)),
            );
        }

        let editor: Element<'_, Message> = match self.adv_sub {
            AdvSubTab::RappySnappy | AdvSubTab::Socd => {
                let (title, desc) = if self.adv_sub == AdvSubTab::Socd {
                    (
                        l.t("Editor de Par SOCD", "SOCD Pair Editor"),
                        l.t(
                            "Resolve direções opostas simultâneas (ex: A + D), priorizando a última tecla pressionada.",
                            "Resolves simultaneous opposite cardinal directions (e.g. A + D) using last-input priority.",
                        ),
                    )
                } else {
                    (
                        l.t("Editor Rappy Snappy (RS)", "Rappy Snappy (RS) Pair Editor"),
                        l.t(
                            "Compara duas teclas e ativa apenas a que estiver pressionada mais fundo no momento.",
                            "Monitors two keys and prioritizes whichever key is currently pressed deeper.",
                        ),
                    )
                };
                container(
                    column![
                        text(title).size(15).color(TEXT_PRIMARY),
                        text(desc).size(12).color(TEXT_MUTED),
                        row![
                            text(l.t("Tecla 1 (linha, col):", "Key 1 (row, col):")).size(12).color(TEXT_SECONDARY),
                            text_input("row", &self.adv_k1_r)
                                .on_input(Message::AdvKey1RowChanged)
                                .width(Length::Fixed(60.0))
                                .style(input_style),
                            text_input("col", &self.adv_k1_c)
                                .on_input(Message::AdvKey1ColChanged)
                                .width(Length::Fixed(60.0))
                                .style(input_style),
                            Space::with_width(Length::Fixed(16.0)),
                            text(l.t("Tecla 2 (linha, col):", "Key 2 (row, col):")).size(12).color(TEXT_SECONDARY),
                            text_input("row", &self.adv_k2_r)
                                .on_input(Message::AdvKey2RowChanged)
                                .width(Length::Fixed(60.0))
                                .style(input_style),
                            text_input("col", &self.adv_k2_c)
                                .on_input(Message::AdvKey2ColChanged)
                                .width(Length::Fixed(60.0))
                                .style(input_style),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        slider_row(
                            format!("{} ({:.2} mm)", l.t("Limiar", "Threshold"), self.adv_apc),
                            0.10..=3.50,
                            self.adv_apc.min(3.50),
                            0.05,
                            Message::AdvApcChanged,
                        ),
                        toggle_row(
                            l.t("Rapid Trigger no par", "Rapid Trigger on Pair"),
                            self.adv_rt,
                            Message::AdvRtToggled,
                        ),
                        row![
                            button(text(l.t("Salvar Par", "Save Pair")).size(12))
                                .padding([6, 14])
                                .style(primary_button_style)
                                .on_press(Message::SaveAdvancedKey),
                            button(text(l.t("Limpar Slot", "Clear Slot")).size(12))
                                .padding([6, 14])
                                .style(danger_button_style)
                                .on_press(Message::ClearAdvancedKey),
                        ]
                        .spacing(8),
                    ]
                    .spacing(10)
                    .padding(18),
                )
                .width(Length::Fill)
                .style(card_style)
                .into()
            }
            AdvSubTab::HoldClick => container(
                column![
                    text("Hold / Click (Tap Dance / MT)").size(15).color(TEXT_PRIMARY),
                    text(l.t(
                        "Executa um código de tecla no toque rápido (Tap) e outro ao manter pressionado (Hold).",
                        "Fires one keycode on quick tap and a different keycode when held past the timer."
                    ))
                    .size(12)
                    .color(TEXT_MUTED),
                    row![
                        text(l.t("Keycode Tap (hex):", "Tap Keycode (hex):")).size(12).color(TEXT_SECONDARY),
                        text_input("0x0004", &self.mt_tap)
                            .on_input(Message::MtTapChanged)
                            .width(Length::Fixed(100.0))
                            .style(input_style),
                        text(l.t("Keycode Hold (hex):", "Hold Keycode (hex):")).size(12).color(TEXT_SECONDARY),
                        text_input("0x00E1", &self.mt_hold)
                            .on_input(Message::MtHoldChanged)
                            .width(Length::Fixed(100.0))
                            .style(input_style),
                        text("Timer (ms):").size(12).color(TEXT_SECONDARY),
                        text_input("200", &self.mt_timer)
                            .on_input(Message::MtTimerChanged)
                            .width(Length::Fixed(80.0))
                            .style(input_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    button(text(l.t("Salvar Slot MT", "Save Hold/Click Slot")).size(12))
                        .padding([6, 14])
                        .style(primary_button_style)
                        .on_press(Message::SaveTapDance),
                ]
                .spacing(10)
                .padding(18),
            )
            .width(Length::Fill)
            .style(card_style)
            .into(),
            AdvSubTab::Dks => {
                let mut rows_col = Column::new().spacing(8);
                for (i, act) in self.dks_draft.actions.iter().enumerate() {
                    let kc_str = format!("{:#06x}", act.keycode());
                    let st_str = format!("{:#06x}", act.status);
                    let kc_clone = kc_str.clone();
                    let st_clone = st_str.clone();
                    rows_col = rows_col.push(
                        row![
                            text(format!("{} {i}:", l.t("Ação", "Action"))).size(12).color(TEXT_SECONDARY),
                            text_input("keycode", &kc_str)
                                .on_input(move |s| Message::DksActionChanged(i, s, st_clone.clone()))
                                .width(Length::Fixed(100.0))
                                .style(input_style),
                            text(l.t("Máscara:", "Trigger Mask:")).size(12).color(TEXT_SECONDARY),
                            text_input("mask", &st_str)
                                .on_input(move |s| Message::DksActionChanged(i, kc_clone.clone(), s))
                                .width(Length::Fixed(100.0))
                                .style(input_style),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    );
                }
                container(
                    column![
                        text("Dynamic Key Stroke (DKS)").size(15).color(TEXT_PRIMARY),
                        text(l.t(
                            "Dispara até 4 ações diferentes em pontos distintos de descida e subida de uma tecla.",
                            "Triggers up to 4 distinct actions across press and release travel points."
                        ))
                        .size(12)
                        .color(TEXT_MUTED),
                        rows_col,
                        button(text(l.t("Salvar Slot DKS", "Save DKS Slot")).size(12))
                            .padding([6, 14])
                            .style(primary_button_style)
                            .on_press(Message::SaveDks),
                    ]
                    .spacing(10)
                    .padding(18),
                )
                .width(Length::Fill)
                .style(card_style)
                .into()
            }
            AdvSubTab::Toggle => container(
                column![
                    text("Toggle Switch (TGL)").size(15).color(TEXT_PRIMARY),
                    text(l.t(
                        "TGL não é suportado pelo firmware DuckBread (protocolo v9).",
                        "TGL is not supported by DuckBread firmware protocol v9."
                    ))
                    .size(12)
                    .color(TEXT_MUTED),
                ]
                .spacing(8)
                .padding(18),
            )
            .width(Length::Fill)
            .style(card_style)
            .into(),
        };

        column![sub_tabs, slot_row, editor].spacing(12).into()
    }

    fn view_app_trigger(&self) -> Element<'_, Message> {
        let l = self.lang;
        let none_lbl = l.t("(Nenhum)", "(None)").to_string();
        let mut prof_choices = vec![none_lbl.clone()];
        prof_choices.extend(self.profiles.iter().cloned());
        let cur_default = self
            .switcher_cfg
            .default_profile
            .clone()
            .unwrap_or(none_lbl);

        let status_card = container(
            column![
                row![
                    column![
                        text(l.t("Troca Automática por App", "Auto-Switcher")).size(15).color(TEXT_PRIMARY),
                        text(l.t(
                            "Aplica automaticamente o perfil vinculado ao focar uma janela (sem desgaste de memória flash).",
                            "Automatically switches profiles in RAM when a matching app window gains focus (zero flash wear)."
                        ))
                        .size(12)
                        .color(TEXT_MUTED),
                    ]
                    .spacing(3),
                    Space::with_width(Length::Fixed(12.0)),
                    toggler(self.switcher_cfg.enabled).on_toggle(Message::ToggleAutoSwitcher),
                    Space::with_width(Length::Fill),
                    container(
                        text(format!("Backend: {}", watcher_backend_name()))
                            .size(11)
                            .color(TEXT_SECONDARY)
                    )
                    .padding([4, 10])
                    .style(badge_style(ACCENT_BLUE)),
                ]
                .align_y(Alignment::Center),
                row![
                    text(l.t("App em foco agora:", "Foreground App:")).size(12).color(TEXT_SECONDARY),
                    text(
                        self.snap
                            .foreground_exe
                            .as_deref()
                            .filter(|s| !s.is_empty())
                            .unwrap_or("—")
                    )
                    .size(12)
                    .color(ACCENT_BLUE),
                    Space::with_width(Length::Fill),
                    text(l.t("Perfil Padrão (Fallback):", "Default Fallback:")).size(12).color(TEXT_SECONDARY),
                    pick_list(prof_choices, Some(cur_default), Message::SetDefaultProfile)
                        .style(pick_list_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(12)
            .padding(18),
        )
        .width(Length::Fill)
        .style(card_style);

        let add_card = container(
            column![
                text(format!(
                    "{}{}",
                    l.t("Vincular App ao Perfil", "Bind App to Profile"),
                    self.selected_profile
                        .as_deref()
                        .map(|p| format!(" ({p})"))
                        .unwrap_or_default()
                ))
                .size(15)
                .color(TEXT_PRIMARY),
                row![
                    text_input(
                        l.t("Executável (ex: cs2, vivaldi-bin)", "Executable (e.g. cs2, firefox)"),
                        &self.trigger_exe
                    )
                    .on_input(Message::TriggerExeChanged)
                    .style(input_style),
                    text_input(
                        l.t("Título da janela (opcional)", "Window Title (optional)"),
                        &self.trigger_title
                    )
                    .on_input(Message::TriggerTitleChanged)
                    .style(input_style),
                    button(text(l.t("Adicionar Regra", "Add Rule")).size(12))
                        .padding([7, 14])
                        .style(primary_button_style)
                        .on_press(Message::AddTriggerRule),
                ]
                .spacing(8),
                row![
                    text(l.t("Janelas Abertas (clique para preencher):", "Running Windows (click to fill):"))
                        .size(12)
                        .color(TEXT_SECONDARY),
                    Space::with_width(Length::Fill),
                    button(text(l.t("Atualizar Janelas", "Refresh Processes")).size(11))
                        .padding([4, 10])
                        .style(secondary_button_style)
                        .on_press(Message::RefreshProcesses),
                ]
                .align_y(Alignment::Center),
                {
                    let mut proc_row = Row::new().spacing(6);
                    for p in self.processes.iter().take(16) {
                        proc_row = proc_row.push(
                            button(text(p.exe.clone()).size(11))
                                .padding([4, 9])
                                .style(secondary_button_style)
                                .on_press(Message::PickRunningProcess(p.exe.clone())),
                        );
                    }
                    scrollable(proc_row).direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::default(),
                    ))
                }
            ]
            .spacing(10)
            .padding(18),
        )
        .width(Length::Fill)
        .style(card_style);

        let mut rules_col = Column::new().spacing(6);
        if self.switcher_cfg.rules.is_empty() {
            rules_col = rules_col.push(
                text(l.t("Nenhuma regra configurada ainda.", "No rules configured yet."))
                    .size(12)
                    .color(TEXT_MUTED),
            );
        }
        for (i, r) in self.switcher_cfg.rules.iter().enumerate() {
            rules_col = rules_col.push(
                container(
                    row![
                        text(&r.profile).size(12).color(ACCENT_BLUE),
                        text(format!("exe: {}   title: {}", r.exe, r.title))
                            .size(12)
                            .color(TEXT_PRIMARY),
                        Space::with_width(Length::Fill),
                        button(text(l.t("Remover", "Remove")).size(11))
                            .padding([3, 8])
                            .style(danger_button_style)
                            .on_press(Message::RemoveTriggerRule(i)),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .padding([8, 12]),
                )
                .style(elevated_card_style),
            );
        }

        let rules_card = container(
            column![
                text(l.t("Regras Configuradas", "Configured Rules"))
                    .size(15)
                    .color(TEXT_PRIMARY),
                rules_col,
            ]
            .spacing(8)
            .padding(18),
        )
        .width(Length::Fill)
        .style(card_style);

        let _ = TriggerBinding::default();
        column![status_card, add_card, rules_card].spacing(12).into()
    }

    fn view_other_settings(&self) -> Element<'_, Message> {
        let l = self.lang;
        let info_card = container(
            column![
                text(l.t("Diagnóstico, Backup e Memória Flash", "Diagnostics, Backup & Flash"))
                    .size(15)
                    .color(TEXT_PRIMARY),
                text(l.t(
                    "No uso diário os ajustes ficam na RAM do teclado. Use \"Salvar na Flash\" para gravar permanentemente ou exporte um backup JSON completo.",
                    "Daily tweaks apply to volatile RAM. Use Save to Flash to persist across power cycles, or export a full JSON backup."
                ))
                .size(12)
                .color(TEXT_MUTED),
                text(format!(
                    "{}: {} (PID {:#06x}) · {}: {}x{} · {}: {:?} · Firmware: {:?}",
                    l.t("Placa", "Board"),
                    self.snap.board.name,
                    self.snap.board.product_id,
                    l.t("Matriz", "Geometry"),
                    self.snap.board.rows,
                    self.snap.board.cols,
                    l.t("Protocolo", "Protocol"),
                    self.snap.protocol_version,
                    self.snap.firmware_version,
                ))
                .size(12)
                .color(TEXT_SECONDARY),
                row![
                    button(text(l.t("Salvar na Flash", "Save to Flash")).size(12))
                        .padding([7, 14])
                        .style(primary_button_style)
                        .on_press(Message::CommitToFlash),
                    button(text(l.t("Exportar Backup", "Export Backup")).size(12))
                        .padding([7, 14])
                        .style(secondary_button_style)
                        .on_press(Message::ExportBackup),
                    button(text(l.t("Restaurar Backup", "Restore Backup")).size(12))
                        .padding([7, 14])
                        .style(secondary_button_style)
                        .on_press(Message::RestoreBackup),
                    button(text(l.t("Reportar Placa", "Report Board")).size(12))
                        .padding([7, 14])
                        .style(secondary_button_style)
                        .on_press(Message::OpenGithubIssue),
                ]
                .spacing(8),
            ]
            .spacing(10)
            .padding(18),
        )
        .width(Length::Fill)
        .style(card_style);

        let danger_card = container(
            column![
                text(l.t("Factory Reset (Memória Interna do Teclado)", "Factory Reset (Onboard Memory)"))
                    .size(15)
                    .color(RED_DANGER),
                text(l.t(
                    "Digite RESET abaixo para confirmar a limpeza da calibração e perfil salvo no teclado.",
                    "Type RESET below to confirm wiping onboard calibration and profile memory."
                ))
                .size(12)
                .color(TEXT_MUTED),
                row![
                    text_input(l.t("Digite RESET", "Type RESET"), &self.factory_confirm)
                        .on_input(Message::FactoryResetInputChanged)
                        .width(Length::Fixed(180.0))
                        .style(input_style),
                    button(text("Factory Reset").size(12))
                        .padding([7, 14])
                        .style(danger_button_style)
                        .on_press(Message::ConfirmFactoryReset),
                ]
                .spacing(10),
            ]
            .spacing(10)
            .padding(18),
        )
        .width(Length::Fill)
        .style(card_style);

        column![info_card, danger_card].spacing(12).into()
    }

    fn view_macros(&self) -> Element<'_, Message> {
        let l = self.lang;
        let mut slots = Row::new().spacing(4);
        for i in 0..16 {
            slots = slots.push(
                button(text(format!("M{i}")).size(11))
                    .padding([5, 9])
                    .style(tab_button_style(self.macro_slot == i))
                    .on_press(Message::SelectMacroSlot(i)),
            );
        }

        let kinds = vec![
            "tap".to_string(),
            "down".to_string(),
            "up".to_string(),
            "delay".to_string(),
            "text".to_string(),
        ];

        let add_row = row![
            pick_list(kinds, Some(self.macro_step_kind.clone()), Message::MacroStepKindChanged)
                .style(pick_list_style),
            text_input(
                l.t("Keycode (0x04) / ms / texto", "Keycode (0x04) / ms / text"),
                &self.macro_step_val
            )
            .on_input(Message::MacroStepValChanged)
            .style(input_style),
            button(text(l.t("Adicionar Passo", "Add Step")).size(12))
                .padding([7, 14])
                .style(secondary_button_style)
                .on_press(Message::AddMacroStep),
            Space::with_width(Length::Fill),
            button(text(l.t("Salvar Slot", "Save Slot")).size(12))
                .padding([7, 14])
                .style(primary_button_style)
                .on_press(Message::SaveMacroSlot),
            button(text(l.t("Limpar", "Clear")).size(12))
                .padding([7, 14])
                .style(danger_button_style)
                .on_press(Message::ClearMacroSlot),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut steps_col = Column::new().spacing(6);
        for (idx, step) in self.macro_steps.iter().enumerate() {
            let desc = match step {
                MacroStep::Tap { keycode } => format!("Tap  {}", keycodes::label(*keycode)),
                MacroStep::Down { keycode } => format!("Down {}", keycodes::label(*keycode)),
                MacroStep::Up { keycode } => format!("Up   {}", keycodes::label(*keycode)),
                MacroStep::Delay { delay_ms } => format!("Delay {delay_ms} ms"),
                MacroStep::Text { text } => format!("Text \"{text}\""),
            };
            steps_col = steps_col.push(
                container(
                    row![
                        text(format!("{idx:02}. {desc}")).size(12).color(TEXT_PRIMARY),
                        Space::with_width(Length::Fill),
                        button(text(l.t("Remover", "Remove")).size(11))
                            .padding([3, 8])
                            .style(danger_button_style)
                            .on_press(Message::RemoveMacroStep(idx)),
                    ]
                    .align_y(Alignment::Center)
                    .padding([6, 12]),
                )
                .style(elevated_card_style),
            );
        }

        container(
            column![
                text(l.t(
                    "Macros Onboard (16 Slots Compartilhados)",
                    "Onboard Macros (16 Shared Slots)"
                ))
                .size(16)
                .color(TEXT_PRIMARY),
                text(l.t(
                    "Macros ficam armazenadas na memória interna do teclado e são compartilhadas por todos os perfis.",
                    "Macros live in the keyboard's onboard memory and are shared across every profile."
                ))
                .size(12)
                .color(TEXT_MUTED),
                slots,
                add_row,
                steps_col,
            ]
            .spacing(12)
            .padding(18),
        )
        .width(Length::Fill)
        .style(card_style)
        .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let l = self.lang;
        let upd_text = match &self.update_info {
            None => format!("{} v{APP_VERSION}", l.t("Versão atual:", "Running")),
            Some(Ok(u)) if u.available => format!(
                "{}: v{} ({} v{})",
                l.t("Atualização disponível", "Update available"),
                u.latest,
                l.t("atual", "current"),
                u.current
            ),
            Some(Ok(u)) => format!("{} (v{})", l.t("Atualizado", "Up to date"), u.current),
            Some(Err(e)) => e.clone(),
        };

        let langs = vec!["Português (BR)".to_string(), "English".to_string()];
        let cur_lang = if l.is_pt() {
            "Português (BR)".to_string()
        } else {
            "English".to_string()
        };

        let card = container(
            column![
                row![
                    text(l.t("Configurações", "Settings")).size(18).color(TEXT_PRIMARY),
                    Space::with_width(Length::Fill),
                    button(text(l.t("Voltar", "Back")).size(12))
                        .padding([6, 12])
                        .style(secondary_button_style)
                        .on_press(Message::ToggleSettingsModal(false)),
                ]
                .align_y(Alignment::Center),
                row![
                    text(l.t("Idioma da Interface", "Interface Language"))
                        .size(13)
                        .color(TEXT_PRIMARY),
                    Space::with_width(Length::Fill),
                    pick_list(langs, Some(cur_lang), Message::SelectLanguage).style(pick_list_style),
                ]
                .align_y(Alignment::Center),
                toggle_row(
                    l.t("Verificar atualizações ao iniciar", "Check for updates on startup"),
                    self.settings.check_updates,
                    Message::CheckUpdatesPrefToggled,
                ),
                toggle_row(
                    l.t("Iniciar junto com o sistema", "Start at login"),
                    self.settings.start_at_login,
                    Message::StartAtLoginToggled,
                ),
                {
                    let has_upd = matches!(&self.update_info, Some(Ok(u)) if u.available);
                    let mut upd_row = row![
                        text(upd_text).size(12).color(TEXT_SECONDARY),
                        Space::with_width(Length::Fill),
                        button(text(l.t("Verificar Agora", "Check Now")).size(12))
                            .padding([6, 12])
                            .style(secondary_button_style)
                            .on_press(Message::CheckUpdates),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center);
                    if has_upd {
                        upd_row = upd_row.push(
                            button(text(l.t("Instalar Atualização", "Install Update")).size(12))
                                .padding([6, 12])
                                .style(primary_button_style)
                                .on_press(Message::InstallUpdate),
                        );
                    }
                    upd_row
                },
                column![
                    text(l.t("Calibração de Cor dos LEDs (Ganho RGB)", "LED Channel Color Calibration (Gain)"))
                        .size(15)
                        .color(TEXT_PRIMARY),
                    text(l.t(
                        "Equilibra o balanço de branco dos LEDs em tempo real (verde e azul costumam ser mais fortes que o vermelho).",
                        "Balances LED white point in real time (green/blue diodes are naturally brighter than red)."
                    ))
                    .size(12)
                    .color(TEXT_MUTED),
                ]
                .spacing(3),
                slider_row(
                    format!("{} ({:.2})", l.t("Ganho Vermelho", "Red Gain"), self.settings.led_gain[0]),
                    0.1..=1.0,
                    self.settings.led_gain[0],
                    0.01,
                    Message::GainRChanged,
                ),
                slider_row(
                    format!("{} ({:.2})", l.t("Ganho Verde", "Green Gain"), self.settings.led_gain[1]),
                    0.1..=1.0,
                    self.settings.led_gain[1],
                    0.01,
                    Message::GainGChanged,
                ),
                slider_row(
                    format!("{} ({:.2})", l.t("Ganho Azul", "Blue Gain"), self.settings.led_gain[2]),
                    0.1..=1.0,
                    self.settings.led_gain[2],
                    0.01,
                    Message::GainBChanged,
                ),
                row![
                    button(text(l.t("Brilho Máximo Original (1.0, 1.0, 1.0)", "Original Max Brightness (1.0, 1.0, 1.0)")).size(12))
                        .padding([6, 12])
                        .style(primary_button_style)
                        .on_press(Message::SetGainPreset([1.0, 1.0, 1.0])),
                    button(text(l.t("Balanço Suave (1.0, 0.65, 0.75)", "Soft White Balance (1.0, 0.65, 0.75)")).size(12))
                        .padding([6, 12])
                        .style(secondary_button_style)
                        .on_press(Message::SetGainPreset([1.0, 0.65, 0.75])),
                ]
                .spacing(8),
            ]
            .spacing(12)
            .padding(20),
        )
        .width(Length::Fill)
        .style(card_style);

        container(scrollable(card))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .into()
    }
}

fn slider_row<'a>(
    label: String,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    step: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label)
            .size(12)
            .color(TEXT_SECONDARY)
            .width(Length::Fixed(195.0)),
        slider(range, value, on_change)
            .step(step)
            .style(slider_style),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn toggle_row<'a>(
    label: &'a str,
    val: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label).size(13).color(TEXT_PRIMARY),
        Space::with_width(Length::Fill),
        toggler(val).on_toggle(on_toggle),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn toggle_desc_row<'a>(
    label: &'a str,
    desc: &'a str,
    val: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        column![
            text(label).size(13).color(TEXT_PRIMARY),
            text(desc).size(11).color(TEXT_MUTED),
        ]
        .spacing(2),
        Space::with_width(Length::Fill),
        toggler(val).on_toggle(on_toggle),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn parse_u16_hex_or_dec(s: &str) -> Option<u16> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u16::from_str_radix(hex, 16).ok()
    } else {
        t.parse::<u16>().ok()
    }
}

fn parse_hex_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 || !t.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&t[0..2], 16).ok()?;
    let g = u8::from_str_radix(&t[2..4], 16).ok()?;
    let b = u8::from_str_radix(&t[4..6], 16).ok()?;
    Some((r, g, b))
}
