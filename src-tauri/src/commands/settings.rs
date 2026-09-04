use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use crate::db::{models::{Settings, SettingsPatch}, queries};
use crate::shortcuts::manager;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let conn = state.db.lock();
    queries::get_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<AppState>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    // Sync launch-at-login with the autostart plugin whenever the flag changes
    if let Some(enable) = patch.launch_at_login {
        let autostart = app.autolaunch();
        let _ = if enable { autostart.enable() } else { autostart.disable() };
    }

    // Grab the previous master shortcut before it's overwritten, so we can
    // unregister it below if it's changing.
    let old_master_shortcut = {
        let conn = state.db.lock();
        queries::get_settings(&conn).ok().map(|s| s.master_shortcut)
    };

    let updated = {
        let conn = state.db.lock();
        queries::update_settings(&conn, &patch).map_err(|e| e.to_string())?
    };

    // The OS-level hotkey grab is only wired up at startup (register_all_shortcuts)
    // and on folder edits — without this, saving a new master shortcut here would
    // update the database but never take effect until the app restarts.
    //
    // On the Wayland portal path the master shortcut isn't X11-registered at all
    // (see register_startup_shortcuts), and the portal doesn't support silently
    // rebinding on demand — the compositor owns the binding once made, so a
    // change here only takes effect after MonoClip restarts and re-runs the
    // portal bind with the new preferred trigger.
    if let (Some(new_shortcut), Some(old_shortcut)) = (&patch.master_shortcut, &old_master_shortcut) {
        if new_shortcut != old_shortcut {
            if manager::using_wayland_portal() {
                log::info!(
                    "Master shortcut changed to '{}' — takes effect after restarting \
                     MonoClip (this session binds it once at startup via the system portal).",
                    new_shortcut
                );
            } else {
                let _ = manager::unregister_shortcut(&app, old_shortcut);
                if let Err(e) = manager::register_master_shortcut(&app, new_shortcut) {
                    log::error!("Failed to register new master shortcut '{}': {}", new_shortcut, e);
                }
            }
        }
    }

    Ok(updated)
}
