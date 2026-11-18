//! Cross-platform system tray icon and dynamic bilingual quick menu.

use crate::icon::generate_app_icon_rgba;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
#[cfg(target_os = "linux")]
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
#[cfg(not(target_os = "linux"))]
use tray_icon::TrayIcon;
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayAction {
    ShowWindow,
    HideWindow,
    ToggleSwitcher,
    CommitToFlash,
    ApplyProfile(String),
    Quit,
}

#[derive(Clone, Default)]
struct TrayMenuIds {
    show_id: Option<MenuId>,
    hide_id: Option<MenuId>,
    toggle_id: Option<MenuId>,
    flash_id: Option<MenuId>,
    quit_id: Option<MenuId>,
    profile_ids: Vec<(MenuId, String)>,
}

#[derive(Clone)]
pub struct TrayMenuState {
    pub profiles: Vec<String>,
    pub active_profile: Option<String>,
    pub switcher_enabled: bool,
    pub is_pt: bool,
}

pub struct TrayHandle {
    #[cfg(not(target_os = "linux"))]
    tray: TrayIcon,
    #[cfg(target_os = "linux")]
    stop: Arc<AtomicBool>,
    #[cfg(target_os = "linux")]
    update_tx: mpsc::Sender<TrayMenuState>,
    ids: Arc<Mutex<TrayMenuIds>>,
}

#[cfg(target_os = "linux")]
impl Drop for TrayHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn load_tray_icon() -> Option<Icon> {
    let rgba = generate_app_icon_rgba(64);
    Icon::from_rgba(rgba, 64, 64).ok()
}

fn build_menu(st: &TrayMenuState) -> (Menu, TrayMenuIds) {
    let pt = st.is_pt;
    let menu = Menu::new();
    let show_item = MenuItem::new(if pt { "Abrir OpenMAD" } else { "Open OpenMAD" }, true, None);
    let hide_item = MenuItem::new(
        if pt {
            "Ocultar para Bandeja"
        } else {
            "Hide to Tray"
        },
        true,
        None,
    );
    let toggle_lbl = match (pt, st.switcher_enabled) {
        (true, true) => "Troca Automática: Ativa",
        (true, false) => "Troca Automática: Pausada",
        (false, true) => "Auto-Switcher: On",
        (false, false) => "Auto-Switcher: Paused",
    };
    let toggle_item = MenuItem::new(toggle_lbl, true, None);
    let flash_item = MenuItem::new(
        if pt {
            "Salvar na Memória Flash"
        } else {
            "Write to Keyboard Flash"
        },
        true,
        None,
    );
    let quit_item = MenuItem::new(if pt { "Sair" } else { "Quit" }, true, None);

    let mut ids = TrayMenuIds {
        show_id: Some(show_item.id().clone()),
        hide_id: Some(hide_item.id().clone()),
        toggle_id: Some(toggle_item.id().clone()),
        flash_id: Some(flash_item.id().clone()),
        quit_id: Some(quit_item.id().clone()),
        profile_ids: Vec::new(),
    };

    let _ = menu.append(&show_item);
    let _ = menu.append(&hide_item);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&toggle_item);
    let _ = menu.append(&flash_item);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let profiles_sub = Submenu::new(if pt { "Perfis" } else { "Profiles" }, true);
    if st.profiles.is_empty() {
        let empty = MenuItem::new(
            if pt {
                "(Nenhum perfil salvo)"
            } else {
                "(No saved profiles)"
            },
            false,
            None,
        );
        let _ = profiles_sub.append(&empty);
    } else {
        for name in &st.profiles {
            let is_active = st.active_profile.as_deref() == Some(name.as_str());
            let label = if is_active {
                format!("● {name}")
            } else {
                name.clone()
            };
            let item = MenuItem::new(label, true, None);
            ids.profile_ids.push((item.id().clone(), name.clone()));
            let _ = profiles_sub.append(&item);
        }
    }
    let _ = menu.append(&profiles_sub);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit_item);

    (menu, ids)
}

impl TrayHandle {
    #[cfg(not(target_os = "linux"))]
    pub fn new(initial: TrayMenuState) -> Option<Self> {
        let icon = load_tray_icon()?;
        let (menu, menu_ids) = build_menu(&initial);
        let ids = Arc::new(Mutex::new(menu_ids));

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("OpenMAD-Rust")
            .with_icon(icon)
            .build()
            .ok()?;

        Some(Self { tray, ids })
    }

    #[cfg(target_os = "linux")]
    pub fn new(initial: TrayMenuState) -> Option<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let ids = Arc::new(Mutex::new(TrayMenuIds::default()));
        let ids_thread = Arc::clone(&ids);
        let (ready_tx, ready_rx) = mpsc::channel::<bool>();
        let (update_tx, update_rx) = mpsc::channel::<TrayMenuState>();

        std::thread::spawn(move || {
            if gtk::init().is_err() {
                let _ = ready_tx.send(false);
                return;
            }

            let Some(icon) = load_tray_icon() else {
                let _ = ready_tx.send(false);
                return;
            };

            let (menu, menu_ids) = build_menu(&initial);
            if let Ok(mut guard) = ids_thread.lock() {
                *guard = menu_ids;
            }

            let Ok(tray) = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip("OpenMAD-Rust")
                .with_icon(icon)
                .build()
            else {
                let _ = ready_tx.send(false);
                return;
            };

            let _ = ready_tx.send(true);

            while !stop_thread.load(Ordering::Relaxed) {
                while let Ok(next_state) = update_rx.try_recv() {
                    let (new_menu, new_ids) = build_menu(&next_state);
                    if let Ok(mut guard) = ids_thread.lock() {
                        *guard = new_ids;
                    }
                    tray.set_menu(Some(Box::new(new_menu)));
                }
                while gtk::events_pending() {
                    gtk::main_iteration_do(false);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });

        if !ready_rx.recv().ok()? {
            return None;
        }

        Some(Self {
            stop,
            update_tx,
            ids,
        })
    }

    pub fn update_state(&self, state: TrayMenuState) {
        #[cfg(target_os = "linux")]
        {
            let _ = self.update_tx.send(state);
        }
        #[cfg(not(target_os = "linux"))]
        {
            let (new_menu, new_ids) = build_menu(&state);
            if let Ok(mut guard) = self.ids.lock() {
                *guard = new_ids;
            }
            self.tray.set_menu(Some(Box::new(new_menu)));
        }
    }

    pub fn poll_event(&self) -> Option<TrayAction> {
        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Ok(guard) = self.ids.lock() {
                if guard.show_id.as_ref() == Some(&ev.id) {
                    return Some(TrayAction::ShowWindow);
                }
                if guard.hide_id.as_ref() == Some(&ev.id) {
                    return Some(TrayAction::HideWindow);
                }
                if guard.toggle_id.as_ref() == Some(&ev.id) {
                    return Some(TrayAction::ToggleSwitcher);
                }
                if guard.flash_id.as_ref() == Some(&ev.id) {
                    return Some(TrayAction::CommitToFlash);
                }
                if guard.quit_id.as_ref() == Some(&ev.id) {
                    return Some(TrayAction::Quit);
                }
                for (pid, name) in &guard.profile_ids {
                    if pid == &ev.id {
                        return Some(TrayAction::ApplyProfile(name.clone()));
                    }
                }
            }
        }
        if let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                ev,
                TrayIconEvent::Click { .. } | TrayIconEvent::DoubleClick { .. }
            ) {
                return Some(TrayAction::ShowWindow);
            }
        }
        None
    }
}
