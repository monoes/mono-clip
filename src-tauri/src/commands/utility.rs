use tauri::{AppHandle, Emitter, Manager, State};
use crate::db::queries;
use crate::db::models::AppStats;
use crate::state::AppState;

/// Attempt to install `mclip` for the user: symlink into `~/.local/bin/` on Unix,
/// copy into `~\.monoclip\bin\` on Windows. Safe to call on every launch —
/// skips silently if already installed.
pub fn auto_install_cli(app: &AppHandle) {
    #[cfg(unix)]
    {
        let Ok(exe) = std::env::current_exe() else { return };
        let Some(mac_os_dir) = exe.parent() else { return };
        let mclip_bin = mac_os_dir.join("mclip");
        if !mclip_bin.exists() {
            return; // not bundled yet (dev mode)
        }

        let Ok(home) = std::env::var("HOME") else { return };
        let bin_dir = std::path::PathBuf::from(&home).join(".local").join("bin");
        if std::fs::create_dir_all(&bin_dir).is_err() {
            return;
        }
        let link = bin_dir.join("mclip");
        // Already points to the right binary — nothing to do
        if link.read_link().ok().as_deref() == Some(&mclip_bin) {
            return;
        }
        // Remove stale symlink/file then create a fresh one
        let _ = std::fs::remove_file(&link);
        if std::os::unix::fs::symlink(&mclip_bin, &link).is_ok() {
            log::info!("mclip installed → {:?}", link);
            let _ = app.emit("cli:installed", link.to_string_lossy().into_owned());
        }
    }
    #[cfg(target_os = "windows")]
    {
        let Ok(exe) = std::env::current_exe() else { return };
        let Some(exe_dir) = exe.parent() else { return };
        let mclip_bin = exe_dir.join("mclip.exe");
        if !mclip_bin.exists() {
            return; // not bundled yet (dev mode)
        }

        let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else { return };
        let bin_dir = std::path::PathBuf::from(&home).join(".monoclip").join("bin");
        if std::fs::create_dir_all(&bin_dir).is_err() {
            return;
        }
        let dest = bin_dir.join("mclip.exe");
        // Same size — assume already installed, nothing to do
        if let (Ok(bundled), Ok(installed)) = (std::fs::metadata(&mclip_bin), std::fs::metadata(&dest)) {
            if bundled.len() == installed.len() {
                return;
            }
        }
        if std::fs::copy(&mclip_bin, &dest).is_ok() {
            log::info!("mclip installed → {:?}", dest);
            let _ = app.emit("cli:installed", dest.to_string_lossy().into_owned());
        }
    }
}

#[tauri::command]
pub fn get_stats(state: State<AppState>) -> Result<AppStats, String> {
    let conn = state.db.lock();
    queries::get_stats(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn run_auto_cleanup(state: State<AppState>) -> Result<i64, String> {
    let conn = state.db.lock();
    let settings = queries::get_settings(&conn).map_err(|e| e.to_string())?;
    if !settings.auto_clean_enabled {
        return Ok(0);
    }
    queries::run_auto_cleanup(&conn, settings.auto_clean_days).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn install_cli(app: AppHandle) -> Result<String, String> {
    #[cfg(unix)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mac_os_dir = exe.parent().ok_or("cannot find binary directory")?;
        let mclip_bin = mac_os_dir.join("mclip");

        if !mclip_bin.exists() {
            return Err(
                "mclip binary not found. This is expected in dev mode — build a release first.".into(),
            );
        }

        let home = std::env::var("HOME").map_err(|e| e.to_string())?;
        let bin_dir = std::path::PathBuf::from(&home).join(".local").join("bin");
        std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
        let link = bin_dir.join("mclip");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&mclip_bin, &link).map_err(|e| e.to_string())?;

        let path = link.to_string_lossy().into_owned();
        let _ = app.emit("cli:installed", &path);
        return Ok(path);
    }
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_dir = exe.parent().ok_or("cannot find binary directory")?;
        let mclip_bin = exe_dir.join("mclip.exe");

        if !mclip_bin.exists() {
            return Err(
                "mclip binary not found. This is expected in dev mode — build a release first.".into(),
            );
        }

        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .ok_or("could not determine home directory")?;
        let bin_dir = home.join(".monoclip").join("bin");
        std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
        let dest = bin_dir.join("mclip.exe");
        std::fs::copy(&mclip_bin, &dest).map_err(|e| e.to_string())?;

        // Best-effort — never fails the install
        add_bin_dir_to_user_path(&bin_dir);

        let path = dest.to_string_lossy().into_owned();
        let _ = app.emit("cli:installed", &path);
        return Ok(path);
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    Err("CLI installation is not supported on this platform.".into())
}

/// Best-effort append of the bin dir to the user PATH. Uses reg.exe instead of
/// setx (which truncates at 1024 chars). New terminals pick the change up on their own.
#[cfg(target_os = "windows")]
fn add_bin_dir_to_user_path(bin_dir: &std::path::Path) {
    const BIN_DIR_LITERAL: &str = r"%USERPROFILE%\.monoclip\bin";

    let Ok(out) = std::process::Command::new("reg")
        .args(["query", "HKCU\\Environment", "/v", "Path"])
        .output()
    else {
        return;
    };
    if !out.status.success() {
        // No user PATH value yet — create one
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Environment", "/v", "Path", "/t", "REG_EXPAND_SZ", "/d", BIN_DIR_LITERAL, "/f"])
            .output();
        return;
    }

    let decoded = decode_reg_output(&out.stdout);
    let Some(current) = decoded.lines().find_map(reg_path_value) else {
        return;
    };
    let already_present = current.split(';').map(str::trim).any(|entry| {
        entry.eq_ignore_ascii_case(BIN_DIR_LITERAL) || std::path::Path::new(entry) == bin_dir
    });
    if already_present {
        return;
    }

    let mut value = current.trim().to_string();
    if value.is_empty() {
        value = BIN_DIR_LITERAL.to_string();
    } else {
        value.push(';');
        value.push_str(BIN_DIR_LITERAL);
    }
    let _ = std::process::Command::new("reg")
        .args(["add", "HKCU\\Environment", "/v", "Path", "/t", "REG_EXPAND_SZ", "/d", &value, "/f"])
        .output();
}

/// reg.exe emits UTF-16LE when redirected on some Windows versions, ANSI/OEM on others.
#[cfg(target_os = "windows")]
fn decode_reg_output(bytes: &[u8]) -> String {
    let zero_high_bytes = bytes.iter().skip(1).step_by(2).filter(|b| **b == 0).count();
    if zero_high_bytes * 4 > bytes.len() {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units).replace('\u{FEFF}', "")
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(target_os = "windows")]
fn reg_path_value(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let idx = trimmed
        .find("REG_EXPAND_SZ")
        .or_else(|| trimmed.find("REG_SZ"))?;
    Some(trimmed[idx..].split_once(char::is_whitespace).map(|(_, v)| v).unwrap_or("").trim())
}

/// Returns true if the Accessibility permission has been granted to this process.
#[tauri::command]
pub fn check_accessibility() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        unsafe { AXIsProcessTrusted() }
    }
    #[cfg(not(target_os = "macos"))]
    true
}

/// Opens System Settings to the Accessibility privacy pane.
#[tauri::command]
pub fn open_accessibility_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

/// Trigger a full automatic update: download, install, and relaunch.
#[tauri::command]
pub fn do_update(app: AppHandle) -> Result<(), String> {
    crate::updater::apply_update(&app);
    Ok(())
}

#[tauri::command]
pub fn toggle_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
