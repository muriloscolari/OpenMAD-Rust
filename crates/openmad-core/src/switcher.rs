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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FallbackMode {
    /// Revert to `default_profile` whenever the focused window matches no rule.
    #[default]
    OnUnfocus,
    /// Keep the active rule's profile across Alt-Tab and revert only after all matching windows close.
    OnClose,
    /// Never revert automatically when no rule matches.
    Disabled,
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
    pub fallback_mode: FallbackMode,
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
            fallback_mode: FallbackMode::OnUnfocus,
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

#[derive(Default)]
pub struct SwitcherState {
    pub active_profile: Option<String>,
    pub active_rule: Option<Rule>,
    pub candidate_profile: Option<String>,
    pub candidate_since: Option<Instant>,
    pub last_switch: Option<Instant>,
}

impl SwitcherState {
    /// Returns `Some(profile_name)` if debounce & dwell criteria are met and a switch should happen.
    #[allow(dead_code)]
    pub fn evaluate(&mut self, cfg: &SwitcherConfig, app: Option<&ForegroundApp>) -> Option<String> {
        self.evaluate_with_windows(cfg, app, &[])
    }

    /// Evaluates active window + currently open windows (needed for [`FallbackMode::OnClose`]).
    pub fn evaluate_with_windows(
        &mut self,
        cfg: &SwitcherConfig,
        app: Option<&ForegroundApp>,
        open_windows: &[WindowedProcess],
    ) -> Option<String> {
        if !cfg.enabled {
            return None;
        }
        let matched_rule = app.and_then(|a| cfg.rules.iter().find(|r| r.matches(a)).cloned());
        let (target, next_rule) = if let Some(rule) = matched_rule {
            (rule.profile.clone(), Some(rule))
        } else {
            let fallback = cfg
                .default_profile
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            match cfg.fallback_mode {
                FallbackMode::Disabled => {
                    self.candidate_profile = None;
                    self.candidate_since = None;
                    return None;
                }
                FallbackMode::OnUnfocus => (fallback?, None),
                FallbackMode::OnClose => {
                    let Some(active_rule) = self.active_rule.as_ref() else {
                        self.candidate_profile = None;
                        self.candidate_since = None;
                        return None;
                    };
                    let still_open = open_windows.iter().any(|w| {
                        let fg = ForegroundApp {
                            exe: w.exe.clone(),
                            path: w.path.clone(),
                            title: w.title.clone(),
                        };
                        active_rule.matches(&fg)
                    });
                    if still_open {
                        self.candidate_profile = None;
                        self.candidate_since = None;
                        return None;
                    }
                    (fallback?, None)
                }
            }
        };

        if self.active_profile.as_deref() == Some(target.as_str()) {
            self.active_rule = next_rule;
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
        self.active_rule = next_rule;
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

/// Monotonic counter incremented when an event-driven backend (KDE KWin D-Bus) updates state.
pub fn foreground_change_seq() -> u64 {
    platform::foreground_change_seq()
}

/// Non-blocking Win32 message pump used by headless background tray mode on Windows.
pub fn pump_win32_messages() {
    platform::pump_win32_messages();
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
        DispatchMessageW, EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };

    pub fn backend_name() -> &'static str {
        "Windows (Win32)"
    }

    pub fn foreground_change_seq() -> u64 {
        0
    }

    pub fn pump_win32_messages() {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, OnceLock};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window};
    use x11rb::rust_connection::RustConnection;

    static KDE_LAST: Mutex<Option<ForegroundApp>> = Mutex::new(None);
    static KDE_RAW_TSV: Mutex<(u64, String)> = Mutex::new((0, String::new()));
    static KDE_WINDOWS_CACHE: Mutex<(u64, Vec<WindowedProcess>)> = Mutex::new((0, Vec::new()));
    static KDE_SEQ: AtomicU64 = AtomicU64::new(0);
    static KWIN_BRIDGE: OnceLock<Option<KWinBridgeState>> = OnceLock::new();
    static CACHED_X11: Mutex<Option<CachedX11>> = Mutex::new(None);

    struct CachedX11 {
        conn: RustConnection,
        root: Window,
        active_atom: Atom,
        client_list_atom: Atom,
        wm_name_atom: Atom,
        utf8_atom: Atom,
        pid_atom: Atom,
    }

    pub fn foreground_change_seq() -> u64 {
        if is_kde_wayland() && ensure_kwin_bridge().is_some() {
            KDE_SEQ.load(Ordering::Relaxed).max(1)
        } else {
            0
        }
    }

    pub fn pump_win32_messages() {}

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

            let seq = KDE_SEQ.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
            if let Ok(mut guard) = KDE_RAW_TSV.lock() {
                guard.0 = seq;
                guard.1 = windows_tsv;
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

    fn is_hyprland() -> bool {
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    }

    fn is_sway_or_i3() -> bool {
        std::env::var_os("SWAYSOCK").is_some() || std::env::var_os("I3SOCK").is_some()
    }

    fn hyprland_socket_path() -> Option<std::path::PathBuf> {
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
        if let Some(xdg) = std::env::var_os("XDG_RUNTIME_DIR") {
            let p = std::path::PathBuf::from(xdg)
                .join("hypr")
                .join(&sig)
                .join(".socket.sock");
            if p.exists() {
                return Some(p);
            }
        }
        let tmp = std::path::PathBuf::from("/tmp/hypr")
            .join(sig)
            .join(".socket.sock");
        if tmp.exists() {
            Some(tmp)
        } else {
            None
        }
    }

    fn hyprland_query(cmd: &str) -> Option<serde_json::Value> {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;
        use std::time::Duration;

        let path = hyprland_socket_path()?;
        let mut stream = UnixStream::connect(path).ok()?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(150)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(150)));
        stream.write_all(cmd.as_bytes()).ok()?;
        let mut raw = String::new();
        stream.read_to_string(&mut raw).ok()?;
        serde_json::from_str(&raw).ok()
    }

    fn hyprland_foreground() -> Option<ForegroundApp> {
        let v = hyprland_query("j/activewindow")?;
        let pid = v.get("pid").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let cls = v.get("class").and_then(|x| x.as_str()).unwrap_or("");
        let title = v.get("title").and_then(|x| x.as_str()).unwrap_or("");
        let info = resolve_window_entry(pid, cls, title);
        if info.exe.is_empty() && info.title.is_empty() {
            None
        } else {
            Some(ForegroundApp {
                exe: info.exe,
                path: info.path,
                title: info.title,
            })
        }
    }

    fn hyprland_windows(map: &mut BTreeMap<String, WindowedProcess>) {
        let Some(serde_json::Value::Array(items)) = hyprland_query("j/clients") else {
            return;
        };
        for item in items {
            let pid = item.get("pid").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            let cls = item.get("class").and_then(|x| x.as_str()).unwrap_or("");
            let title = item.get("title").and_then(|x| x.as_str()).unwrap_or("");
            let info = resolve_window_entry(pid, cls, title);
            if !info.exe.is_empty() {
                map.entry(info.exe.to_lowercase()).or_insert(info);
            }
        }
    }

    fn sway_get_tree() -> Option<serde_json::Value> {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;
        use std::time::Duration;

        let sock = std::env::var_os("SWAYSOCK").or_else(|| std::env::var_os("I3SOCK"))?;
        let mut stream = UnixStream::connect(sock).ok()?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(150)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(150)));

        // i3-ipc header: "i3-ipc" + payload_len (u32 ne) + msg_type=4 (GET_TREE, u32 ne)
        let mut req = [0u8; 14];
        req[0..6].copy_from_slice(b"i3-ipc");
        req[6..10].copy_from_slice(&0u32.to_ne_bytes());
        req[10..14].copy_from_slice(&4u32.to_ne_bytes());
        stream.write_all(&req).ok()?;

        let mut hdr = [0u8; 14];
        stream.read_exact(&mut hdr).ok()?;
        if &hdr[0..6] != b"i3-ipc" {
            return None;
        }
        let len = u32::from_ne_bytes([hdr[6], hdr[7], hdr[8], hdr[9]]) as usize;
        if len > 8 * 1024 * 1024 {
            return None;
        }
        let mut body = vec![0u8; len];
        stream.read_exact(&mut body).ok()?;
        serde_json::from_slice(&body).ok()
    }

    fn sway_node_info(node: &serde_json::Value) -> WindowedProcess {
        let pid = node.get("pid").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let app_id = node
            .get("app_id")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                node.get("window_properties")
                    .and_then(|wp| wp.get("class"))
                    .and_then(|x| x.as_str())
            })
            .unwrap_or("");
        let title = node.get("name").and_then(|x| x.as_str()).unwrap_or("");
        resolve_window_entry(pid, app_id, title)
    }

    fn sway_find_focused(node: &serde_json::Value) -> Option<ForegroundApp> {
        if node.get("focused").and_then(|x| x.as_bool()) == Some(true) {
            let info = sway_node_info(node);
            if !info.exe.is_empty() || !info.title.is_empty() {
                return Some(ForegroundApp {
                    exe: info.exe,
                    path: info.path,
                    title: info.title,
                });
            }
        }
        for key in ["nodes", "floating_nodes"] {
            if let Some(arr) = node.get(key).and_then(|x| x.as_array()) {
                for child in arr {
                    if let Some(found) = sway_find_focused(child) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    fn sway_collect_windows(node: &serde_json::Value, map: &mut BTreeMap<String, WindowedProcess>) {
        let is_leaf = node
            .get("pid")
            .and_then(|x| x.as_u64())
            .is_some_and(|p| p > 0)
            || node
                .get("app_id")
                .and_then(|x| x.as_str())
                .is_some_and(|s| !s.is_empty());
        if is_leaf {
            let info = sway_node_info(node);
            if !info.exe.is_empty() {
                map.entry(info.exe.to_lowercase()).or_insert(info);
            }
        }
        for key in ["nodes", "floating_nodes"] {
            if let Some(arr) = node.get(key).and_then(|x| x.as_array()) {
                for child in arr {
                    sway_collect_windows(child, map);
                }
            }
        }
    }

    pub fn backend_name() -> &'static str {
        if is_hyprland() {
            "Linux (Hyprland)"
        } else if is_sway_or_i3() {
            if std::env::var_os("WAYLAND_DISPLAY").is_some() {
                "Linux (Sway)"
            } else {
                "Linux (i3)"
            }
        } else if std::env::var_os("WAYLAND_DISPLAY").is_some() {
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
        if is_hyprland() {
            if let Some(app) = hyprland_foreground() {
                return Some(app);
            }
        }
        if is_sway_or_i3() {
            if let Some(tree) = sway_get_tree() {
                if let Some(app) = sway_find_focused(&tree) {
                    return Some(app);
                }
            }
        }
        // Native KDE Plasma 6 Wayland KWin D-Bus bridge (event-driven; never spawn kdotool or open X11 if active).
        if is_kde_wayland() {
            if ensure_kwin_bridge().is_some() {
                return KDE_LAST.lock().ok().and_then(|g| g.clone());
            }
            if let Some(app) = kdotool_foreground() {
                return Some(app);
            }
        }
        // Fallback to X11 (_NET_ACTIVE_WINDOW) — cached persistent connection.
        x11_foreground()
    }

    fn kdotool_foreground() -> Option<ForegroundApp> {
        let out = std::process::Command::new("kdotool")
            .args(["getactivewindow", "getwindowclassname"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let exe = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let title = std::process::Command::new("kdotool")
            .args(["getactivewindow", "getwindowname"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        if !exe.is_empty() || !title.is_empty() {
            Some(ForegroundApp {
                exe: exe.clone(),
                path: exe,
                title,
            })
        } else {
            None
        }
    }

    fn with_x11<T, F: FnOnce(&CachedX11) -> Option<T>>(f: F) -> Option<T> {
        let mut guard = CACHED_X11.lock().ok()?;
        if guard.is_none() {
            let (conn, screen_num) = x11rb::connect(None).ok()?;
            let root = conn.setup().roots[screen_num].root;
            let active_atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?.reply().ok()?.atom;
            let client_list_atom = conn.intern_atom(false, b"_NET_CLIENT_LIST").ok()?.reply().ok()?.atom;
            let wm_name_atom = conn.intern_atom(false, b"_NET_WM_NAME").ok()?.reply().ok()?.atom;
            let utf8_atom = conn.intern_atom(false, b"UTF8_STRING").ok()?.reply().ok()?.atom;
            let pid_atom = conn.intern_atom(false, b"_NET_WM_PID").ok()?.reply().ok()?.atom;
            *guard = Some(CachedX11 {
                conn,
                root,
                active_atom,
                client_list_atom,
                wm_name_atom,
                utf8_atom,
                pid_atom,
            });
        }
        let res = f(guard.as_ref()?);
        if res.is_none() {
            // Check if connection is still alive; drop cache if broken
            if let Some(ref c) = *guard {
                if c.conn.get_input_focus().is_err() {
                    *guard = None;
                }
            }
        }
        res
    }

    fn x11_foreground() -> Option<ForegroundApp> {
        with_x11(|x| {
            let prop = x
                .conn
                .get_property(false, x.root, x.active_atom, AtomEnum::WINDOW, 0, 1)
                .ok()?
                .reply()
                .ok()?;
            let win: Window = prop.value32()?.next()?;
            if win == 0 {
                return None;
            }
            x11_window_info(x, win)
        })
    }

    fn x11_window_info(x: &CachedX11, win: Window) -> Option<ForegroundApp> {
        let title = x
            .conn
            .get_property(false, win, x.wm_name_atom, x.utf8_atom, 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| String::from_utf8_lossy(&r.value).into_owned())
            .unwrap_or_default();

        let pid = x
            .conn
            .get_property(false, win, x.pid_atom, AtomEnum::CARDINAL, 0, 1)
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
        let class_prop = x
            .conn
            .get_property(false, win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
            .ok()?
            .reply()
            .ok()?;
        let raw = String::from_utf8_lossy(&class_prop.value);
        let exe = raw
            .split('\0')
            .rfind(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        Some(ForegroundApp {
            exe: exe.clone(),
            path: exe,
            title,
        })
    }

    fn kde_wayland_windows_cached() -> Vec<WindowedProcess> {
        if KDE_SEQ.load(Ordering::Acquire) == 0 {
            if let Some(state) = ensure_kwin_bridge() {
                if let Ok(script_proxy) = zbus::blocking::Proxy::new(
                    &state.conn,
                    "org.kde.KWin",
                    state.script_obj_path.as_str(),
                    "org.kde.kwin.Script",
                ) {
                    let _: std::result::Result<(), _> = script_proxy.call("run", &());
                }
                for _ in 0..5 {
                    if KDE_SEQ.load(Ordering::Acquire) > 0 {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
            }
        }

        let cur_seq = KDE_SEQ.load(Ordering::Acquire);
        if let Ok(cache) = KDE_WINDOWS_CACHE.lock() {
            if cache.0 == cur_seq && cur_seq > 0 {
                return cache.1.clone();
            }
        }

        let (tsv_seq, tsv) = KDE_RAW_TSV
            .lock()
            .map(|g| (g.0, g.1.clone()))
            .unwrap_or_default();
        let mut map: BTreeMap<String, WindowedProcess> = BTreeMap::new();
        for line in tsv.lines() {
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
        let vec: Vec<WindowedProcess> = map.into_values().collect();
        if let Ok(mut cache) = KDE_WINDOWS_CACHE.lock() {
            cache.0 = tsv_seq;
            cache.1 = vec.clone();
        }
        vec
    }

    pub fn list_windowed_processes() -> Vec<WindowedProcess> {
        let mut map: BTreeMap<String, WindowedProcess> = BTreeMap::new();
        let mut handled_wayland = false;

        if is_hyprland() {
            hyprland_windows(&mut map);
            handled_wayland = !map.is_empty();
        }

        if is_sway_or_i3() {
            if let Some(tree) = sway_get_tree() {
                sway_collect_windows(&tree, &mut map);
                handled_wayland = true;
            }
        }

        if !handled_wayland && is_kde_wayland() && ensure_kwin_bridge().is_some() {
            for p in kde_wayland_windows_cached() {
                map.entry(p.exe.to_lowercase()).or_insert(p);
            }
            handled_wayland = true;
        }

        // Enumerate X11/XWayland client list only if a native Wayland backend did not handle it.
        if !handled_wayland {
            let _ = with_x11(|x| {
                let prop = x
                    .conn
                    .get_property(false, x.root, x.client_list_atom, AtomEnum::WINDOW, 0, 4096)
                    .ok()?
                    .reply()
                    .ok()?;
                if let Some(wins) = prop.value32() {
                    for win in wins {
                        if let Some(app) = x11_window_info(x, win) {
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
                Some(())
            });
        }
        // Scan /proc fallback only if no backend returned anything
        if !handled_wayland && map.is_empty() {
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
    pub fn foreground_change_seq() -> u64 { 0 }
    pub fn pump_win32_messages() {}
    pub fn foreground_app() -> Option<ForegroundApp> { None }
    pub fn list_windowed_processes() -> Vec<WindowedProcess> { Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_matching_and_glob() {
        let r = Rule {
            profile: "FPS".into(),
            exe: "cs2*".into(),
            title: "*Counter-Strike*".into(),
        };
        let app_ok = ForegroundApp {
            exe: "cs2.exe".into(),
            path: "/games/cs2.exe".into(),
            title: "Counter-Strike 2".into(),
        };
        let app_wrong_title = ForegroundApp {
            exe: "cs2.exe".into(),
            path: "".into(),
            title: "Settings".into(),
        };
        assert!(r.matches(&app_ok));
        assert!(!r.matches(&app_wrong_title));

        let empty_rule = Rule {
            profile: "X".into(),
            exe: "".into(),
            title: "".into(),
        };
        assert!(!empty_rule.matches(&app_ok));
    }

    #[test]
    fn test_switcher_config_resolve_and_debounce_dwell() {
        let cfg = SwitcherConfig {
            enabled: true,
            default_profile: Some("Default".into()),
            rules: vec![Rule {
                profile: "Browser".into(),
                exe: "firefox".into(),
                title: String::new(),
            }],
            poll_ms: 100,
            debounce_ms: 0,
            min_dwell_ms: 0,
            ..Default::default()
        };

        let ff = ForegroundApp {
            exe: "firefox".into(),
            path: String::new(),
            title: "Docs".into(),
        };
        assert_eq!(cfg.resolve(Some(&ff)).as_deref(), Some("Browser"));
        assert_eq!(cfg.resolve(None).as_deref(), Some("Default"));

        let mut state = SwitcherState::default();
        // First tick records candidate; second tick (after debounce_ms=0) activates
        assert_eq!(state.evaluate(&cfg, Some(&ff)), None);
        assert_eq!(state.evaluate(&cfg, Some(&ff)).as_deref(), Some("Browser"));
        // Same target does not re-trigger
        assert_eq!(state.evaluate(&cfg, Some(&ff)), None);
    }

    #[test]
    fn test_switcher_fallback_on_close_and_disabled() {
        let mut cfg = SwitcherConfig {
            enabled: true,
            default_profile: Some("Padrão".into()),
            fallback_mode: FallbackMode::OnClose,
            rules: vec![Rule {
                profile: "Jogo".into(),
                exe: "cs2".into(),
                title: String::new(),
            }],
            poll_ms: 100,
            debounce_ms: 0,
            min_dwell_ms: 0,
            ..Default::default()
        };

        let game = ForegroundApp {
            exe: "cs2".into(),
            path: String::new(),
            title: "Counter-Strike 2".into(),
        };
        let browser = ForegroundApp {
            exe: "vivaldi-bin".into(),
            path: String::new(),
            title: "Docs".into(),
        };
        let game_win = vec![WindowedProcess {
            exe: "cs2".into(),
            path: String::new(),
            title: "Counter-Strike 2".into(),
        }];

        let mut state = SwitcherState::default();
        // Activate rule for game
        assert_eq!(state.evaluate_with_windows(&cfg, Some(&game), &game_win), None);
        assert_eq!(
            state.evaluate_with_windows(&cfg, Some(&game), &game_win).as_deref(),
            Some("Jogo")
        );

        // Alt-Tab to browser while game is still open: keep "Jogo"
        assert_eq!(
            state.evaluate_with_windows(&cfg, Some(&browser), &game_win),
            None
        );
        assert_eq!(
            state.evaluate_with_windows(&cfg, Some(&browser), &game_win),
            None
        );

        // Close the game (no matching window left): revert to "Padrão"
        assert_eq!(state.evaluate_with_windows(&cfg, Some(&browser), &[]), None);
        assert_eq!(
            state.evaluate_with_windows(&cfg, Some(&browser), &[]).as_deref(),
            Some("Padrão")
        );

        // Disabled mode never reverts
        cfg.fallback_mode = FallbackMode::Disabled;
        assert_eq!(state.evaluate_with_windows(&cfg, Some(&game), &game_win), None);
        assert_eq!(
            state.evaluate_with_windows(&cfg, Some(&game), &game_win).as_deref(),
            Some("Jogo")
        );
        assert_eq!(state.evaluate_with_windows(&cfg, Some(&browser), &[]), None);
        assert_eq!(state.evaluate_with_windows(&cfg, Some(&browser), &[]), None);
    }
}
