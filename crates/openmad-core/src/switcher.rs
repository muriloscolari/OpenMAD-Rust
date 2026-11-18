//! Application-aware profile switching for Windows and Linux (KDE Wayland + X11).

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundApp {
    pub exe: String,
    pub path: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowedProcess {
    pub exe: String,
    pub path: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub profile: String,
    #[serde(default)]
    pub exe: String,
    #[serde(default)]
    pub title: String,
}

impl Rule {
    pub fn matches(&self, app: &ForegroundApp) -> bool {
        let exe_ok = if self.exe.trim().is_empty() {
            true
        } else {
            glob_match(&self.exe.to_lowercase(), &app.exe.to_lowercase())
        };
        let title_ok = if self.title.trim().is_empty() {
            true
        } else {
            glob_match(&self.title.to_lowercase(), &app.title.to_lowercase())
        };
        exe_ok && title_ok && (!self.exe.trim().is_empty() || !self.title.trim().is_empty())
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') && !pattern.contains('?') {
        return text == pattern || text.contains(pattern);
    }
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;
    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    for i in 1..=p.len() {
        for j in 1..=t.len() {
            if p[i - 1] == '*' {
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
            } else if p[i - 1] == '?' || p[i - 1] == t[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            }
        }
    }
    dp[p.len()][t.len()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitcherConfig {
    #[serde(default = "default_cfg_ver")]
    pub version: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default = "default_poll_ms")]
    pub poll_ms: u64,
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_dwell_ms")]
    pub min_dwell_ms: u64,
}

fn default_cfg_ver() -> u32 { CONFIG_VERSION }
fn default_true() -> bool { true }
fn default_poll_ms() -> u64 { 250 }
fn default_debounce_ms() -> u64 { 400 }
fn default_dwell_ms() -> u64 { 800 }

impl Default for SwitcherConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            enabled: true,
            default_profile: None,
            rules: Vec::new(),
            poll_ms: 250,
            debounce_ms: 400,
            min_dwell_ms: 800,
        }
    }
}

impl SwitcherConfig {
    pub fn load(path: &Path) -> Self {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(cfg) = serde_json::from_str::<Self>(text.trim_start_matches('\u{feff}')) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn resolve(&self, app: Option<&ForegroundApp>) -> Option<String> {
        if let Some(a) = app {
            for r in &self.rules {
                if r.matches(a) {
                    return Some(r.profile.clone());
                }
            }
        }
        self.default_profile.clone().filter(|s| !s.trim().is_empty())
    }
}

pub struct SwitcherState {
    pub active_profile: Option<String>,
    pub candidate_profile: Option<String>,
    pub candidate_since: Option<Instant>,
    pub last_switch: Option<Instant>,
}

impl Default for SwitcherState {
    fn default() -> Self {
        Self {
            active_profile: None,
            candidate_profile: None,
            candidate_since: None,
            last_switch: None,
        }
    }
}

impl SwitcherState {
    /// Returns `Some(profile_name)` if debounce & dwell criteria are met and a switch should happen.
    pub fn evaluate(&mut self, cfg: &SwitcherConfig, app: Option<&ForegroundApp>) -> Option<String> {
        if !cfg.enabled {
            return None;
        }
        let target = cfg.resolve(app)?;
        if self.active_profile.as_deref() == Some(target.as_str()) {
            self.candidate_profile = None;
            self.candidate_since = None;
            return None;
        }
        let now = Instant::now();
        if self.candidate_profile.as_deref() != Some(target.as_str()) {
            self.candidate_profile = Some(target);
            self.candidate_since = Some(now);
            return None;
        }
        let stable_ms = self
            .candidate_since
            .map(|t| now.duration_since(t).as_millis() as u64)
            .unwrap_or(0);
        if stable_ms < cfg.debounce_ms {
            return None;
        }
        if let Some(last) = self.last_switch {
            if (now.duration_since(last).as_millis() as u64) < cfg.min_dwell_ms {
                return None;
            }
        }
        self.active_profile = Some(target.clone());
        self.last_switch = Some(now);
        self.candidate_profile = None;
        self.candidate_since = None;
        Some(target)
    }
}

/// Platform-specific foreground window inspection.
pub fn foreground_app() -> Option<ForegroundApp> {
    platform::foreground_app()
}

/// Platform-specific list of visible windowed processes for the App Trigger picker.
pub fn list_windowed_processes() -> Vec<WindowedProcess> {
    platform::list_windowed_processes()
}

/// Human-readable description of the active window detection backend.
pub fn watcher_backend_name() -> &'static str {
    platform::backend_name()
}

#[cfg(target_os = "windows")]
mod platform {
    use super::{ForegroundApp, WindowedProcess};
    use std::collections::BTreeMap;
    use std::path::Path;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, MAX_PATH};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible,
    };

    pub fn backend_name() -> &'static str {
        "Windows (Win32)"
    }

    fn hwnd_info(hwnd: HWND) -> Option<(String, String, String)> {
        unsafe {
            let len = GetWindowTextLengthW(hwnd);
            let mut title_buf = vec![0u16; (len as usize) + 1];
            let read = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..read as usize]);

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return Some((String::new(), String::new(), title));
            }
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return Some((String::new(), String::new(), title));
            };
            let mut path_buf = vec![0u16; MAX_PATH as usize * 4];
            let mut size = path_buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(path_buf.as_mut_ptr()),
                &mut size,
            )
            .is_ok();
            let _ = windows::Win32::Foundation::CloseHandle(handle);

            let path = if ok {
                String::from_utf16_lossy(&path_buf[..size as usize])
            } else {
                String::new()
            };
            let exe = Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            Some((exe, path, title))
        }
    }

    pub fn foreground_app() -> Option<ForegroundApp> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }
            let (exe, path, title) = hwnd_info(hwnd)?;
            Some(ForegroundApp { exe, path, title })
        }
    }

    pub fn list_windowed_processes() -> Vec<WindowedProcess> {
        let mut map: BTreeMap<String, WindowedProcess> = BTreeMap::new();
        unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }
            if GetWindowTextLengthW(hwnd) <= 0 {
                return BOOL(1);
            }
            let map = &mut *(lparam.0 as *mut BTreeMap<String, WindowedProcess>);
            if let Some((exe, path, title)) = hwnd_info(hwnd) {
                if !exe.is_empty() && !map.contains_key(&exe.to_lowercase()) {
                    map.insert(
                        exe.to_lowercase(),
                        WindowedProcess { exe, path, title },
                    );
                }
            }
            BOOL(1)
        }
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut map as *mut _ as isize));
        }
        map.into_values().collect()
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{ForegroundApp, WindowedProcess};
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, Window};

    static KDE_LAST: Mutex<Option<ForegroundApp>> = Mutex::new(None);
    static KDE_WINDOWS: Mutex<Vec<WindowedProcess>> = Mutex::new(Vec::new());
    static KWIN_BRIDGE: OnceLock<Option<KWinBridgeState>> = OnceLock::new();

    struct KWinBridgeState {
        conn: zbus::blocking::Connection,
        script_obj_path: String,
    }

    struct KWinBridge;

    #[zbus::interface(name = "org.openmad.KWinBridge")]
    impl KWinBridge {
        #[zbus(name = "Update")]
        fn update(
            &self,
            pid_str: String,
            resource_class: String,
            caption: String,
            windows_tsv: String,
        ) {
            let pid = pid_str.trim().parse::<u32>().unwrap_or(0);
            let active = resolve_window_entry(pid, &resource_class, &caption);
            if let Ok(mut guard) = KDE_LAST.lock() {
                *guard = if active.exe.is_empty() && active.title.is_empty() {
                    None
                } else {
                    Some(ForegroundApp {
                        exe: active.exe,
                        path: active.path,
                        title: active.title,
                    })
                };
            }

            let mut map: BTreeMap<String, WindowedProcess> = BTreeMap::new();
            for line in windows_tsv.lines() {
                let mut parts = line.splitn(3, '\t');
                let p_str = parts.next().unwrap_or("0");
                let r_cls = parts.next().unwrap_or("");
                let cap = parts.next().unwrap_or("");
                let w_pid = p_str.trim().parse::<u32>().unwrap_or(0);
                let proc_info = resolve_window_entry(w_pid, r_cls, cap);
                if !proc_info.exe.is_empty() {
                    map.entry(proc_info.exe.to_lowercase()).or_insert(proc_info);
                }
            }
            if let Ok(mut guard) = KDE_WINDOWS.lock() {
                *guard = map.into_values().collect();
            }
        }
    }

    fn resolve_window_entry(pid: u32, resource_class: &str, caption: &str) -> WindowedProcess {
        if pid > 0 {
            let exe_path = std::fs::read_link(format!("/proc/{pid}/exe"))
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let exe_from_proc = std::path::Path::new(&exe_path)
                .file_name()
                .and_then(|s| s.to_str())
                .map(str::to_string)
                .or_else(|| {
                    std::fs::read_to_string(format!("/proc/{pid}/comm"))
                        .ok()
                        .map(|s| s.trim().to_string())
                })
                .unwrap_or_default();
            if !exe_from_proc.is_empty() {
                return WindowedProcess {
                    exe: exe_from_proc,
                    path: exe_path,
                    title: caption.to_string(),
                };
            }
        }
        let fallback = resource_class.trim().to_string();
        WindowedProcess {
            exe: fallback.clone(),
            path: fallback,
            title: caption.to_string(),
        }
    }

    fn is_kde_wayland() -> bool {
        std::env::var_os("WAYLAND_DISPLAY").is_some()
            && std::env::var("XDG_CURRENT_DESKTOP")
                .unwrap_or_default()
                .to_uppercase()
                .contains("KDE")
    }

    fn ensure_kwin_bridge() -> Option<&'static KWinBridgeState> {
        KWIN_BRIDGE.get_or_init(start_kwin_bridge).as_ref()
    }

    fn start_kwin_bridge() -> Option<KWinBridgeState> {
        let conn = zbus::blocking::connection::Builder::session()
            .ok()?
            .name("org.openmad.KWinBridge")
            .ok()?
            .serve_at("/OpenMADKWin", KWinBridge)
            .ok()?
            .build()
            .ok()?;

        let script_js = r#"
function esc(s) {
    return String(s || "").replace(/[\t\r\n]+/g, " ");
}
function pushState() {
    var act = workspace.activeWindow;
    var pid = act ? String(act.pid || 0) : "0";
    var cls = act ? esc(act.resourceClass) : "";
    var cap = act ? esc(act.caption) : "";
    var list = workspace.windowList ? workspace.windowList() : [];
    var lines = [];
    for (var i = 0; i < list.length; i++) {
        var w = list[i];
        if (!w || w.specialWindow) continue;
        lines.push(String(w.pid || 0) + "\t" + esc(w.resourceClass) + "\t" + esc(w.caption));
    }
    callDBus("org.openmad.KWinBridge", "/OpenMADKWin", "org.openmad.KWinBridge", "Update",
             pid, cls, cap, lines.join("\n"));
}
workspace.windowActivated.connect(function() { pushState(); });
if (workspace.windowAdded) workspace.windowAdded.connect(function() { pushState(); });
if (workspace.windowRemoved) workspace.windowRemoved.connect(function() { pushState(); });
pushState();
"#;

        let script_path = std::env::temp_dir().join("openmad_kwin_bridge.js");
        std::fs::write(&script_path, script_js).ok()?;
        let script_path_str = script_path.to_string_lossy().into_owned();

        let scripting = zbus::blocking::Proxy::new(
            &conn,
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting",
        )
        .ok()?;

        let _: std::result::Result<bool, _> = scripting.call("unloadScript", &("openmad_kwin_bridge",));
        let script_id: i32 = scripting
            .call("loadScript", &(script_path_str.as_str(), "openmad_kwin_bridge"))
            .ok()?;
        if script_id < 0 {
            return None;
        }
        let obj_path = format!("/Scripting/Script{script_id}");
        if let Ok(script_proxy) = zbus::blocking::Proxy::new(
            &conn,
            "org.kde.KWin",
            obj_path.as_str(),
            "org.kde.kwin.Script",
        ) {
            let _: std::result::Result<(), _> = script_proxy.call("run", &());
        }

        Some(KWinBridgeState {
            conn,
            script_obj_path: obj_path,
        })
    }

    pub fn backend_name() -> &'static str {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            if std::env::var("XDG_CURRENT_DESKTOP")
                .unwrap_or_default()
                .to_uppercase()
                .contains("KDE")
            {
                "Linux (KDE Wayland)"
            } else {
                "Linux (Wayland / XWayland)"
            }
        } else {
            "Linux (X11)"
        }
    }

    pub fn foreground_app() -> Option<ForegroundApp> {
        // 1. Try KDE KWin D-Bus Scripting bridge (native on KDE Plasma 6 Wayland).
        if is_kde_wayland() {
            if let Some(app) = kde_wayland_foreground() {
                return Some(app);
            }
        }
        // 2. Fallback to X11 (_NET_ACTIVE_WINDOW) — also covers XWayland apps if DISPLAY is set.
        x11_foreground()
    }

    fn kde_wayland_foreground() -> Option<ForegroundApp> {
        let _ = ensure_kwin_bridge();
        if let Ok(guard) = KDE_LAST.lock() {
            if let Some(ref app) = *guard {
                return Some(app.clone());
            }
        }
        if let Ok(out) = std::process::Command::new("kdotool")
            .args(["getactivewindow", "getwindowclassname"])
            .output()
        {
            if out.status.success() {
                let exe = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let title = std::process::Command::new("kdotool")
                    .args(["getactivewindow", "getwindowname"])
                    .output()
                    .ok()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();
                if !exe.is_empty() || !title.is_empty() {
                    return Some(ForegroundApp {
                        exe: exe.clone(),
                        path: exe,
                        title,
                    });
                }
            }
        }
        None
    }

    fn x11_foreground() -> Option<ForegroundApp> {
        let (conn, screen_num) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots[screen_num].root;
        let active_atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?.reply().ok()?.atom;
        let prop = conn
            .get_property(false, root, active_atom, AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?;
        let win: Window = prop.value32()?.next()?;
        if win == 0 {
            return None;
        }
        x11_window_info(&conn, win)
    }

    fn x11_window_info<C: Connection>(conn: &C, win: Window) -> Option<ForegroundApp> {
        let wm_name = conn.intern_atom(false, b"_NET_WM_NAME").ok()?.reply().ok()?.atom;
        let utf8 = conn.intern_atom(false, b"UTF8_STRING").ok()?.reply().ok()?.atom;
        let title = conn
            .get_property(false, win, wm_name, utf8, 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| String::from_utf8_lossy(&r.value).into_owned())
            .unwrap_or_default();

        let pid_atom = conn.intern_atom(false, b"_NET_WM_PID").ok()?.reply().ok()?.atom;
        let pid = conn
            .get_property(false, win, pid_atom, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32()?.next());

        if let Some(pid) = pid {
            let exe_path = std::fs::read_link(format!("/proc/{pid}/exe"))
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let exe = std::path::Path::new(&exe_path)
                .file_name()
                .and_then(|s| s.to_str())
                .map(str::to_string)
                .or_else(|| {
                    std::fs::read_to_string(format!("/proc/{pid}/comm"))
                        .ok()
                        .map(|s| s.trim().to_string())
                })
                .unwrap_or_default();
            return Some(ForegroundApp {
                exe,
                path: exe_path,
                title,
            });
        }

        // Fallback to WM_CLASS
        let class_prop = conn
            .get_property(false, win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
            .ok()?
            .reply()
            .ok()?;
        let raw = String::from_utf8_lossy(&class_prop.value);
        let exe = raw
            .split('\0')
            .filter(|s| !s.is_empty())
            .last()
            .unwrap_or("")
            .to_string();
        Some(ForegroundApp {
            exe: exe.clone(),
            path: exe,
            title,
        })
    }

    pub fn list_windowed_processes() -> Vec<WindowedProcess> {
        let mut map: BTreeMap<String, WindowedProcess> = BTreeMap::new();

        if is_kde_wayland() {
            if let Some(state) = ensure_kwin_bridge() {
                if let Ok(script_proxy) = zbus::blocking::Proxy::new(
                    &state.conn,
                    "org.kde.KWin",
                    state.script_obj_path.as_str(),
                    "org.kde.kwin.Script",
                ) {
                    let _: std::result::Result<(), _> = script_proxy.call("run", &());
                }
                for _ in 0..4 {
                    if let Ok(guard) = KDE_WINDOWS.lock() {
                        if !guard.is_empty() {
                            for p in guard.iter() {
                                map.entry(p.exe.to_lowercase()).or_insert_with(|| p.clone());
                            }
                            break;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(40));
                }
            }
        }

        // Enumerate X11/XWayland client list if available
        if let Ok((conn, screen_num)) = x11rb::connect(None) {
            let root = conn.setup().roots[screen_num].root;
            if let Ok(atom_r) = conn.intern_atom(false, b"_NET_CLIENT_LIST") {
                if let Ok(atom) = atom_r.reply() {
                    if let Ok(prop_r) = conn.get_property(false, root, atom.atom, AtomEnum::WINDOW, 0, 4096) {
                        if let Ok(prop) = prop_r.reply() {
                            if let Some(wins) = prop.value32() {
                                for win in wins {
                                    if let Some(app) = x11_window_info(&conn, win) {
                                        if !app.exe.is_empty() && !map.contains_key(&app.exe.to_lowercase()) {
                                            map.insert(
                                                app.exe.to_lowercase(),
                                                WindowedProcess {
                                                    exe: app.exe,
                                                    path: app.path,
                                                    title: app.title,
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Also scan /proc for running desktop apps if X11 list is empty (pure Wayland)
        if map.is_empty() {
            if let Ok(entries) = std::fs::read_dir("/proc") {
                for entry in entries.flatten() {
                    let fname = entry.file_name();
                    if !fname.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
                        continue;
                    }
                    let exe_link = entry.path().join("exe");
                    if let Ok(target) = std::fs::read_link(&exe_link) {
                        let path = target.to_string_lossy().into_owned();
                        if path.starts_with("/usr/bin/") || path.starts_with("/home/") {
                            if let Some(exe) = target.file_name().and_then(|s| s.to_str()) {
                                map.entry(exe.to_lowercase()).or_insert_with(|| WindowedProcess {
                                    exe: exe.to_string(),
                                    path,
                                    title: String::new(),
                                });
                            }
                        }
                    }
                }
            }
        }
        map.into_values().collect()
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod platform {
    use super::{ForegroundApp, WindowedProcess};
    pub fn backend_name() -> &'static str { "Unsupported" }
    pub fn foreground_app() -> Option<ForegroundApp> { None }
    pub fn list_windowed_processes() -> Vec<WindowedProcess> { Vec::new() }
}
