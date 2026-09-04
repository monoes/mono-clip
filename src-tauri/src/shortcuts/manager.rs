use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_clipboard_manager::ClipboardExt;
use crate::state::AppState;
use crate::db::queries;
use crate::clipboard::detector;
use std::str::FromStr;

/// Capture the currently selected (highlighted) text in the frontmost app.
///
/// Strategy:
///   1. Save the current clipboard contents
///   2. Write a unique sentinel to the clipboard
///   3. Simulate Cmd+C to copy the selection into the clipboard
///   4. Read the clipboard — if it still matches the sentinel, nothing was selected
///   5. Restore the original clipboard so the user doesn't notice
///
/// The sentinel approach avoids false positives from Universal Clipboard (iCloud /
/// iPhone) which can asynchronously replace clipboard contents between steps,
/// making a simple "did the content change?" check unreliable.
#[cfg(target_os = "macos")]
fn capture_selected_text(app: &AppHandle) -> Option<String> {
    // 1. Save current clipboard
    let original = app.clipboard().read_text().ok().unwrap_or_default();

    // 2. Write a unique sentinel so we can tell if Cmd+C actually wrote something
    let sentinel = format!("__monoclip_sentinel_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos());
    if app.clipboard().write_text(&sentinel).is_err() {
        return None;
    }

    // 3. Simulate Cmd+C in the frontmost app
    let status = std::process::Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to keystroke \"c\" using command down"])
        .status();

    if status.is_err() {
        // Restore original clipboard before returning
        let _ = app.clipboard().write_text(&original);
        return None;
    }

    // 4. Brief pause for the local Cmd+C to land
    std::thread::sleep(std::time::Duration::from_millis(150));

    // 5. Read the clipboard
    let current = app.clipboard().read_text().ok().unwrap_or_default();

    // 6. Restore original clipboard
    let _ = app.clipboard().write_text(&original);

    // If the clipboard still holds our sentinel, Cmd+C didn't fire (nothing selected).
    // Also ignore if the clipboard is empty or matches the sentinel.
    if current.is_empty() || current == sentinel || current.starts_with("__monoclip_sentinel_") {
        None
    } else {
        Some(current)
    }
}

/// Windows variant: same sentinel strategy, but sends Ctrl+C via SendInput instead
/// of osascript, and waits slightly longer for the copy to land.
#[cfg(target_os = "windows")]
fn capture_selected_text(app: &AppHandle) -> Option<String> {
    // 1. Save current clipboard
    let original = app.clipboard().read_text().ok().unwrap_or_default();

    // 2. Write a unique sentinel so we can tell if Ctrl+C actually wrote something
    let sentinel = format!("__monoclip_sentinel_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos());
    if app.clipboard().write_text(&sentinel).is_err() {
        return None;
    }

    // 3. Simulate Ctrl+C in the frontmost app
    if crate::keyboard::send_ctrl_c().is_err() {
        // Restore original clipboard before returning
        let _ = app.clipboard().write_text(&original);
        return None;
    }

    // 4. Brief pause for the local Ctrl+C to land
    std::thread::sleep(std::time::Duration::from_millis(200));

    // 5. Read the clipboard
    let current = app.clipboard().read_text().ok().unwrap_or_default();

    // 6. Restore original clipboard
    let _ = app.clipboard().write_text(&original);

    // If the clipboard still holds our sentinel, Ctrl+C didn't fire (nothing selected).
    // Also ignore if the clipboard is empty or matches the sentinel.
    if current.is_empty() || current == sentinel || current.starts_with("__monoclip_sentinel_") {
        None
    } else {
        Some(current)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn capture_selected_text(_app: &AppHandle) -> Option<String> {
    None
}

/// True when the master shortcut should go through the XDG GlobalShortcuts
/// portal (shortcuts::portal) instead of the X11 grab below — the X11 grab
/// never receives physical keypresses from native-Wayland client windows.
pub fn using_wayland_portal() -> bool {
    cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Show/hide the main window near the cursor. Shared by the X11 global-shortcut
/// path and the Wayland GlobalShortcuts portal path (shortcuts::portal).
pub fn toggle_main_window(app: &AppHandle) {
    crate::window::manager::position_window_near_cursor(app);
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

/// Register whichever shortcuts are appropriate for this platform/session at
/// startup. On Linux under Wayland, the master shortcut is bound via the
/// portal (async, may take a moment and can fail silently if the portal
/// backend doesn't implement GlobalShortcuts yet); folder shortcuts still use
/// the X11 path unconditionally. Everywhere else, everything uses X11.
pub fn register_startup_shortcuts(app: &AppHandle) {
    if using_wayland_portal() {
        if let Err(e) = register_folder_shortcuts_only(app) {
            log::error!("Failed to register folder shortcuts: {}", e);
        }

        #[cfg(target_os = "linux")]
        {
            let master_shortcut = {
                let state = app.state::<AppState>();
                let conn = state.db.lock();
                queries::get_settings(&conn)
                    .map(|s| s.master_shortcut)
                    .unwrap_or_else(|_| "CmdOrCtrl+Shift+V".to_string())
            };

            let portal_app = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::shortcuts::portal::run(portal_app, master_shortcut).await;
            });
        }
        return;
    }

    if let Err(e) = register_all_shortcuts(app) {
        log::error!("Failed to register shortcuts: {}", e);
    }
}

/// Register the master (open/close) shortcut via X11. Called when it's changed
/// in Settings — not used on the Wayland portal path (see `using_wayland_portal`).
pub fn register_master_shortcut(app: &AppHandle, shortcut_str: &str) -> anyhow::Result<()> {
    register_shortcut(app, shortcut_str, ShortcutAction::ToggleWindow)
}

/// Register a single folder shortcut. Called when a folder is created or its shortcut updated.
pub fn register_folder_shortcut(
    app: &AppHandle,
    folder_id: i64,
    folder_name: String,
    shortcut_str: &str,
) -> anyhow::Result<()> {
    register_shortcut(app, shortcut_str, ShortcutAction::SaveToFolder { folder_id, folder_name })
}

pub fn register_all_shortcuts(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let (master_shortcut, folder_shortcuts) = {
        let conn = state.db.lock();
        let settings = queries::get_settings(&conn)?;
        let folders = queries::get_folders(&conn)?;
        let folder_shortcuts: Vec<(i64, String, String)> = folders
            .into_iter()
            .filter_map(|f| f.global_shortcut.map(|s| (f.id, f.name, s)))
            .collect();
        (settings.master_shortcut, folder_shortcuts)
    };

    // Register master shortcut (toggle window)
    register_shortcut(app, &master_shortcut, ShortcutAction::ToggleWindow)?;

    // Register folder shortcuts
    for (folder_id, folder_name, shortcut_str) in folder_shortcuts {
        register_shortcut(app, &shortcut_str, ShortcutAction::SaveToFolder { folder_id, folder_name })?;
    }

    Ok(())
}

/// Same as `register_all_shortcuts` but skips the master shortcut — used on
/// the Wayland portal path, where that one is bound separately (see
/// `register_startup_shortcuts`).
fn register_folder_shortcuts_only(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let folder_shortcuts: Vec<(i64, String, String)> = {
        let conn = state.db.lock();
        queries::get_folders(&conn)?
            .into_iter()
            .filter_map(|f| f.global_shortcut.map(|s| (f.id, f.name, s)))
            .collect()
    };

    for (folder_id, folder_name, shortcut_str) in folder_shortcuts {
        register_shortcut(app, &shortcut_str, ShortcutAction::SaveToFolder { folder_id, folder_name })?;
    }

    Ok(())
}

enum ShortcutAction {
    ToggleWindow,
    SaveToFolder { folder_id: i64, folder_name: String },
}

fn register_shortcut(app: &AppHandle, shortcut_str: &str, action: ShortcutAction) -> anyhow::Result<()> {
    let shortcut = match Shortcut::from_str(shortcut_str) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Invalid shortcut '{}': {:?}", shortcut_str, e);
            return Ok(());
        }
    };

    let app_clone = app.clone();
    match action {
        ShortcutAction::ToggleWindow => {
            app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    toggle_main_window(&app_clone);
                }
            })?;
        }
        ShortcutAction::SaveToFolder { folder_id, folder_name } => {
            app.global_shortcut().on_shortcut(shortcut, move |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    // Prefer selected text; fall back to current clipboard
                    let maybe_sel = capture_selected_text(app);
                    let (content, source) = if let Some(sel) = maybe_sel {
                        (sel, "selection")
                    } else {
                        match app.clipboard().read_text() {
                            Ok(c) if !c.is_empty() => (c, "clipboard"),
                            _ => return,
                        }
                    };

                    let state = app.state::<AppState>();
                    let conn = state.db.lock();
                    let content_type = detector::detect_content_type(&content);
                    let preview = detector::make_preview(&content, 200);
                    match queries::insert_clip(&conn, &content, content_type, &preview, folder_id, None) {
                        Ok(clip) => {
                            let _ = app.emit("folder:saved", serde_json::json!({
                                "clip": clip,
                                "folderName": folder_name,
                                "source": source,
                            }));
                        }
                        Err(e) => log::error!("Failed to save clip to folder {}: {}", folder_id, e),
                    }
                }
            })?;
        }
    }

    Ok(())
}

pub fn unregister_shortcut(app: &AppHandle, shortcut_str: &str) -> anyhow::Result<()> {
    if let Ok(shortcut) = Shortcut::from_str(shortcut_str) {
        let _ = app.global_shortcut().unregister(shortcut);
    }
    Ok(())
}
