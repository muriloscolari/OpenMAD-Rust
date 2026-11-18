//! GitHub releases update check and self-updater.

use serde::{Deserialize, Serialize};
use std::io::Read;

pub const APP_NAME: &str = "OpenMAD";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPO_SLUG: &str = "muriloscolari/OpenMAD-Rust";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub url: String,
    pub notes: String,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

pub fn version_tuple(tag: &str) -> Vec<u32> {
    tag.trim()
        .trim_start_matches('v')
        .split('.')
        .filter_map(|p| {
            let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u32>().ok()
        })
        .collect()
}

pub fn check_for_update() -> Result<UpdateInfo, String> {
    let url = format!("https://api.github.com/repos/{REPO_SLUG}/releases/latest");
    let resp = ureq::get(&url)
        .set("User-Agent", "OpenMAD-Rust")
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(6))
        .call()
        .map_err(|e| format!("Update check failed: {e}"))?;

    let rel: GithubRelease = resp
        .into_json()
        .map_err(|e| format!("Invalid release JSON: {e}"))?;

    let cur = version_tuple(APP_VERSION);
    let lat = version_tuple(&rel.tag_name);
    let available = lat > cur;

    Ok(UpdateInfo {
        current: APP_VERSION.to_string(),
        latest: rel.tag_name.trim_start_matches('v').to_string(),
        available,
        url: rel.html_url,
        notes: rel.body,
        assets: rel.assets,
    })
}

fn download_asset(url: &str, dest: &std::path::Path) -> Result<(), String> {
    let resp = ureq::get(url)
        .set("User-Agent", "OpenMAD-Rust")
        .timeout(std::time::Duration::from_secs(120))
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    let mut reader = resp.into_reader();
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|e| format!("Failed reading download stream: {e}"))?;
    std::fs::write(dest, &buf).map_err(|e| format!("Failed writing {}: {e}", dest.display()))
}

#[cfg(target_os = "linux")]
pub fn download_and_apply_update(info: &UpdateInfo) -> Result<String, String> {
    use std::os::unix::fs::PermissionsExt;

    let asset = info
        .assets
        .iter()
        .find(|a| a.name == "openmad-linux-x86_64")
        .ok_or_else(|| "Release asset 'openmad-linux-x86_64' not found".to_string())?;

    let tmp_path = std::env::temp_dir().join("openmad-linux-x86_64.new");
    download_asset(&asset.browser_download_url, &tmp_path)?;
    std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("Failed setting executable permissions: {e}"))?;

    let target = std::env::current_exe().map_err(|e| format!("Cannot resolve current_exe: {e}"))?;
    match replace_executable_linux(&tmp_path, &target) {
        Ok(()) => Ok(format!(
            "Atualizado para v{}! Reinicie o aplicativo para usar a nova versão.",
            info.latest
        )),
        Err(_) => {
            // If running from root-owned /usr/bin/openmad, write to ~/.local/bin/openmad
            if let Some(home) = std::env::var_os("HOME") {
                let user_bin = std::path::PathBuf::from(home).join(".local/bin/openmad");
                if let Some(parent) = user_bin.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if replace_executable_linux(&tmp_path, &user_bin).is_ok() {
                    return Ok(format!(
                        "Atualizado em {} (v{}). Reinicie ou atualize via pacote do sistema.",
                        user_bin.display(),
                        info.latest
                    ));
                }
            }
            Err(format!(
                "Baixado em {}, mas sem permissão para substituir {}. Execute com sudo ou atualize o pacote.",
                tmp_path.display(),
                target.display()
            ))
        }
    }
}

#[cfg(target_os = "linux")]
fn replace_executable_linux(src: &std::path::Path, dest: &std::path::Path) -> std::io::Result<()> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    let staged = dest.with_extension("new");
    std::fs::copy(src, &staged)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&staged, dest)?;
    let _ = std::fs::remove_file(src);
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn download_and_apply_update(info: &UpdateInfo) -> Result<String, String> {
    let setup = info
        .assets
        .iter()
        .find(|a| a.name.starts_with("OpenMAD-Setup-") && a.name.ends_with(".exe"));
    if let Some(asset) = setup {
        let dest = std::env::temp_dir().join(&asset.name);
        download_asset(&asset.browser_download_url, &dest)?;
        std::process::Command::new(&dest)
            .args(["/SILENT", "/CLOSEAPPLICATIONS"])
            .spawn()
            .map_err(|e| format!("Failed launching installer: {e}"))?;
        return Ok(format!(
            "Instalador v{} iniciado em segundo plano.",
            info.latest
        ));
    }

    let exe_asset = info
        .assets
        .iter()
        .find(|a| a.name == "openmad-windows-x86_64.exe")
        .ok_or_else(|| "Nenhum instalador Windows encontrado na release.".to_string())?;
    let dest = std::env::temp_dir().join(&exe_asset.name);
    download_asset(&exe_asset.browser_download_url, &dest)?;
    Ok(format!("Baixado em {}", dest.display()))
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn download_and_apply_update(_info: &UpdateInfo) -> Result<String, String> {
    Err("Self-update is only supported on Linux and Windows".to_string())
}
