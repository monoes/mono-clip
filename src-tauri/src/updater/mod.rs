use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_API: &str = "https://api.github.com/repos/nokhodian/mono-clip/releases/latest";
const CHECK_INTERVAL_SECS: u64 = 3600;

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub tag: String,
    pub download_url: String,
}

/// Holds the latest available update info so `apply_update` can use it without
/// hitting the network again.
static PENDING_UPDATE: OnceLock<Mutex<Option<UpdateInfo>>> = OnceLock::new();

fn pending_update() -> &'static Mutex<Option<UpdateInfo>> {
    PENDING_UPDATE.get_or_init(|| Mutex::new(None))
}

/// Check GitHub for a newer release. Returns UpdateInfo if one exists.
pub fn check_for_update() -> Option<UpdateInfo> {
    let response = ureq::get(RELEASES_API)
        .set("User-Agent", "MonoClip-Updater")
        .call()
        .ok()?;

    let json: serde_json::Value = response.into_json().ok()?;
    let tag = json["tag_name"].as_str()?.to_string();

    let remote = tag.trim_start_matches('v');
    let local = CURRENT_VERSION.trim_start_matches('v');
    if !is_newer(remote, local) {
        return None;
    }

    let download_url = pick_asset_url(&json, &tag);

    Some(UpdateInfo { tag, download_url })
}

fn pick_asset_url(json: &serde_json::Value, tag: &str) -> String {
    let assets = json["assets"].as_array();

    #[cfg(target_os = "windows")]
    {
        let find = |ext: &str| {
            assets.and_then(|list| {
                list.iter().find_map(|a| {
                    let name = a["name"].as_str()?;
                    if name.ends_with(ext) {
                        a["browser_download_url"].as_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            })
        };
        // Prefer NSIS setup.exe, then .msi
        find(".exe")
            .or_else(|| find(".msi"))
            // Fallback: construct URL from tag
            .unwrap_or_else(|| {
                let v = tag.trim_start_matches('v');
                format!(
                    "https://github.com/nokhodian/mono-clip/releases/download/{tag}/MonoClip_{v}_x64-setup.exe"
                )
            })
    }

    #[cfg(target_os = "macos")]
    {
        // Find the aarch64 DMG asset URL
        assets
            .and_then(|list| {
                list.iter().find_map(|a| {
                    let name = a["name"].as_str()?;
                    if name.ends_with(".dmg") {
                        a["browser_download_url"].as_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            })
            // Fallback: construct URL from tag
            .unwrap_or_else(|| {
                let v = tag.trim_start_matches('v');
                format!(
                    "https://github.com/nokhodian/mono-clip/releases/download/{tag}/MonoClip_{v}_aarch64.dmg"
                )
            })
    }

    #[cfg(target_os = "linux")]
    {
        // The AppImage is the only self-updatable Linux artifact — deb/rpm are
        // owned by the package manager, so we never point the updater at them.
        assets
            .and_then(|list| {
                list.iter().find_map(|a| {
                    let name = a["name"].as_str()?;
                    if name.ends_with(".AppImage") {
                        a["browser_download_url"].as_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            })
            // Fallback: construct URL from tag
            .unwrap_or_else(|| {
                let v = tag.trim_start_matches('v');
                format!(
                    "https://github.com/nokhodian/mono-clip/releases/download/{tag}/MonoClip_{v}_amd64.AppImage"
                )
            })
    }
}

fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> (u64, u64, u64) {
        let mut p = s.splitn(3, '.');
        let a = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let b = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        let c = p.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        (a, b, c)
    };
    parse(remote) > parse(local)
}

/// Spawns the hourly background checker.
pub fn start_update_checker<R: Runtime>(app: AppHandle<R>, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(10));

        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }

            if let Some(info) = check_for_update() {
                log::info!("Update available: {}", info.tag);
                // Cache it so apply_update doesn't need to re-fetch
                if let Ok(mut guard) = pending_update().lock() {
                    *guard = Some(info.clone());
                }
                let _ = app.emit("update:available", info.tag);
            }

            let mut elapsed = 0u64;
            while elapsed < CHECK_INTERVAL_SECS {
                std::thread::sleep(Duration::from_secs(60));
                elapsed += 60;
                if stop.load(Ordering::Relaxed) {
                    return;
                }
            }
        }
    });
}

/// Apply the pending update and relaunch the app.
///
/// - Homebrew installs: `brew upgrade --cask mono-clip` then relaunch
/// - Direct installs:   download DMG → mount → copy → unmount → relaunch
/// - Linux AppImage:    download AppImage → swap in place → relaunch
/// - Linux deb/rpm:     no self-update; the package manager owns those files
///
/// Emits `update:progress` with status strings and `update:done` on success.
pub fn apply_update<R: Runtime>(app: &AppHandle<R>) {
    let info = pending_update()
        .lock()
        .ok()
        .and_then(|g| g.clone());

    let Some(info) = info else {
        log::warn!("apply_update called but no pending update cached");
        return;
    };

    let app = app.clone();
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        {
            apply_windows_update(&app, &info);
        }

        #[cfg(target_os = "macos")]
        {
            let exe = std::env::current_exe().unwrap_or_default();
            let path_str = exe.to_string_lossy().to_lowercase();
            let is_homebrew = path_str.contains("caskroom") || path_str.contains("homebrew");

            if is_homebrew {
                apply_homebrew_update(&app);
            } else {
                apply_direct_update(&app, &info);
            }
        }

        #[cfg(target_os = "linux")]
        {
            match appimage_path() {
                Some(path) => apply_appimage_update(&app, &info, &path),
                None => notify_manual_update(&app, &info),
            }
        }
    });
}

fn emit_progress<R: Runtime>(app: &AppHandle<R>, msg: &str) {
    log::info!("[update] {}", msg);
    let _ = app.emit("update:progress", msg);
}

#[cfg(target_os = "windows")]
fn apply_windows_update<R: Runtime>(app: &AppHandle<R>, info: &UpdateInfo) {
    let is_msi = info.download_url.to_lowercase().ends_with(".msi");
    let tmp_installer = std::env::temp_dir().join(if is_msi {
        "MonoClip_setup.msi"
    } else {
        "MonoClip_setup.exe"
    });

    // 1. Download
    emit_progress(app, &format!("Downloading {}…", info.tag));
    let response = match ureq::get(&info.download_url)
        .set("User-Agent", "MonoClip-Updater")
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            let _ = app.emit("update:error", format!("Download failed: {e}"));
            return;
        }
    };

    let mut file = match std::fs::File::create(&tmp_installer) {
        Ok(f) => f,
        Err(e) => {
            let _ = app.emit("update:error", format!("Cannot write temp file: {e}"));
            return;
        }
    };
    if let Err(e) = std::io::copy(&mut response.into_reader(), &mut file) {
        let _ = app.emit("update:error", format!("Download write failed: {e}"));
        return;
    }
    drop(file);

    // 2. Launch installer (NSIS relaunches the app itself when done)
    emit_progress(app, "Launching installer…");
    let launch = if is_msi {
        std::process::Command::new("msiexec")
            .arg("/i")
            .arg(&tmp_installer)
            .spawn()
    } else {
        std::process::Command::new(&tmp_installer).spawn()
    };

    match launch {
        Ok(_) => {
            emit_progress(app, "Installer started — quitting for update…");
            std::thread::sleep(Duration::from_millis(800));
            app.exit(0);
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_installer);
            let _ = app.emit("update:error", format!("Failed to launch installer: {e}"));
        }
    }
}

#[cfg(target_os = "macos")]
fn apply_homebrew_update<R: Runtime>(app: &AppHandle<R>) {
    emit_progress(app, "Running brew upgrade --cask mono-clip…");

    let status = std::process::Command::new("brew")
        .args(["upgrade", "--cask", "mono-clip"])
        .status();

    match status {
        Ok(s) if s.success() => {
            emit_progress(app, "Upgrade complete — relaunching…");
            relaunch(app);
        }
        Ok(s) => {
            log::error!("brew upgrade failed with status {}", s);
            let _ = app.emit("update:error", "brew upgrade failed — please run manually.");
        }
        Err(e) => {
            log::error!("brew upgrade error: {}", e);
            let _ = app.emit("update:error", format!("brew error: {e}"));
        }
    }
}

#[cfg(target_os = "macos")]
fn apply_direct_update<R: Runtime>(app: &AppHandle<R>, info: &UpdateInfo) {
    let tmp_dmg = std::env::temp_dir().join("MonoClip_update.dmg");

    // 1. Download
    emit_progress(app, &format!("Downloading {}…", info.tag));
    let response = match ureq::get(&info.download_url)
        .set("User-Agent", "MonoClip-Updater")
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            let _ = app.emit("update:error", format!("Download failed: {e}"));
            return;
        }
    };

    let mut file = match std::fs::File::create(&tmp_dmg) {
        Ok(f) => f,
        Err(e) => {
            let _ = app.emit("update:error", format!("Cannot write temp file: {e}"));
            return;
        }
    };
    if let Err(e) = std::io::copy(&mut response.into_reader(), &mut file) {
        let _ = app.emit("update:error", format!("Download write failed: {e}"));
        return;
    }
    drop(file);

    // 2. Mount DMG
    emit_progress(app, "Mounting disk image…");
    let mount_out = std::process::Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-quiet", tmp_dmg.to_str().unwrap()])
        .output();

    let mount_point = match mount_out {
        Ok(out) if out.status.success() => {
            // Parse the mount point from output (last field of last line)
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .last()
                .and_then(|l| l.split('\t').last())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "/Volumes/MonoClip".to_string())
        }
        Ok(out) => {
            let _ = app.emit("update:error", format!(
                "hdiutil attach failed: {}", String::from_utf8_lossy(&out.stderr)
            ));
            return;
        }
        Err(e) => {
            let _ = app.emit("update:error", format!("hdiutil error: {e}"));
            return;
        }
    };

    // 3. Copy app
    emit_progress(app, "Installing new version…");
    let app_src = format!("{}/MonoClip.app", mount_point);
    let _ = std::process::Command::new("rm")
        .args(["-rf", "/Applications/MonoClip.app"])
        .status();
    let copy_status = std::process::Command::new("cp")
        .args(["-R", &app_src, "/Applications/MonoClip.app"])
        .status();

    // 4. Unmount
    let _ = std::process::Command::new("hdiutil")
        .args(["detach", &mount_point, "-quiet"])
        .status();
    let _ = std::fs::remove_file(&tmp_dmg);

    match copy_status {
        Ok(s) if s.success() => {
            emit_progress(app, "Installed — relaunching…");
            relaunch(app);
        }
        _ => {
            let _ = app.emit("update:error", "Copy to /Applications failed.");
        }
    }
}

/// Launch a fresh copy of the app then quit this instance.
#[cfg_attr(target_os = "windows", allow(dead_code))]
fn relaunch<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new(std::env::current_exe().unwrap_or_default()).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .args(["-n", "/Applications/MonoClip.app"])
        .spawn();
    // On Linux the AppImage (or the installed binary) re-execs itself directly —
    // there is no `open`-style launcher to hand off to.
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new(
        appimage_path().unwrap_or_else(|| std::env::current_exe().unwrap_or_default()),
    )
    .spawn();
    // Brief pause so the new process can start before we exit
    std::thread::sleep(Duration::from_millis(800));
    app.exit(0);
}

/// Path to the running AppImage, if this process was launched from one.
///
/// The AppImage runtime exports `APPIMAGE` with the absolute path of the
/// `.AppImage` file itself (`current_exe()` instead points inside the
/// throwaway `/tmp/.mount_*` squashfs, which is useless for self-replacement).
#[cfg(target_os = "linux")]
fn appimage_path() -> Option<std::path::PathBuf> {
    let raw = std::env::var_os("APPIMAGE")?;
    let path = std::path::PathBuf::from(raw);
    if path.is_absolute() && path.exists() {
        Some(path)
    } else {
        None
    }
}

/// deb/rpm installs are owned by the package manager — replacing those files
/// behind its back would leave the package database lying. Point the user at
/// the release instead.
#[cfg(target_os = "linux")]
fn notify_manual_update<R: Runtime>(app: &AppHandle<R>, info: &UpdateInfo) {
    let releases_url = format!(
        "https://github.com/nokhodian/mono-clip/releases/tag/{}",
        info.tag
    );
    log::info!("[update] {} available; package-managed install, not self-updating", info.tag);

    // xdg-open is the only launcher we can rely on across desktops.
    if std::process::Command::new("xdg-open")
        .arg(&releases_url)
        .spawn()
        .is_err()
    {
        log::warn!("xdg-open unavailable; user must visit {} manually", releases_url);
    }

    let _ = app.emit(
        "update:manual",
        format!(
            "MonoClip {} is available. This build was installed from a package \
             (deb/rpm), so update it with your package manager or download the \
             new release.",
            info.tag
        ),
    );
}

/// Download the new AppImage and swap it over the running one.
///
/// The replacement is staged next to the target and moved into place with a
/// rename, so an interrupted download can never leave a half-written AppImage
/// where the working one used to be.
#[cfg(target_os = "linux")]
fn apply_appimage_update<R: Runtime>(
    app: &AppHandle<R>,
    info: &UpdateInfo,
    current: &std::path::Path,
) {
    use std::os::unix::fs::PermissionsExt;

    // Staging in the same directory keeps the final step a rename (atomic)
    // rather than a cross-filesystem copy.
    let staging = current.with_extension("AppImage.new");

    emit_progress(app, &format!("Downloading {}…", info.tag));
    let response = match ureq::get(&info.download_url)
        .set("User-Agent", "MonoClip-Updater")
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            let _ = app.emit("update:error", format!("Download failed: {e}"));
            return;
        }
    };

    let mut file = match std::fs::File::create(&staging) {
        Ok(f) => f,
        Err(e) => {
            let _ = app.emit(
                "update:error",
                format!("Cannot write next to the AppImage ({e}) — is it on a read-only mount?"),
            );
            return;
        }
    };
    if let Err(e) = std::io::copy(&mut response.into_reader(), &mut file) {
        let _ = std::fs::remove_file(&staging);
        let _ = app.emit("update:error", format!("Download write failed: {e}"));
        return;
    }
    drop(file);

    // An AppImage that isn't executable is just a file the kernel refuses to run.
    if let Err(e) = std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755)) {
        let _ = std::fs::remove_file(&staging);
        let _ = app.emit("update:error", format!("Cannot mark AppImage executable: {e}"));
        return;
    }

    emit_progress(app, "Installing new version…");
    if let Err(e) = std::fs::rename(&staging, current) {
        let _ = std::fs::remove_file(&staging);
        let _ = app.emit("update:error", format!("Failed to replace AppImage: {e}"));
        return;
    }

    emit_progress(app, "Installed — relaunching…");
    relaunch(app);
}
