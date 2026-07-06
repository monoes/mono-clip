// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use monoclip_lib::{
    commands::{clips, folders, settings, utility},
    db::connection,
    shortcuts::manager as shortcuts_manager,
    state::AppState,
    tray::setup::setup_tray,
    clipboard::watcher,
    updater,
};

fn main() {
    env_logger::init();
    monoclip_lib::crash_report::install("monoclip");

    let db = connection::open_database().expect("Failed to open database");
    let app_state = AppState::new(db);

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle().clone();

            // Menu bar-only app: no Dock icon, no Cmd+Tab entry
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Set up system tray
            setup_tray(&handle)?;

            // Register global shortcuts
            if let Err(e) = shortcuts_manager::register_all_shortcuts(&handle) {
                log::error!("Failed to register shortcuts: {}", e);
            }

            // Start clipboard watcher
            watcher::start_watcher(handle.clone());

            // Start hourly update checker
            let stop_flag = Arc::new(AtomicBool::new(false));
            updater::start_update_checker(handle.clone(), stop_flag);

            // Install mclip CLI tool into ~/.local/bin/ if bundled binary is present
            utility::auto_install_cli(&handle);

            // Sync launch-at-login: make sure the LaunchAgent matches the stored setting
            {
                let state = app.state::<AppState>();
                let conn = state.db.lock();
                if let Ok(settings) = monoclip_lib::db::queries::get_settings(&conn) {
                    let autostart = handle.autolaunch();
                    let _ = if settings.launch_at_login {
                        autostart.enable()
                    } else {
                        autostart.disable()
                    };
                }
            }

            // Run auto-cleanup on startup
            {
                let state = app.state::<AppState>();
                let conn = state.db.lock();
                if let Ok(settings) = monoclip_lib::db::queries::get_settings(&conn) {
                    if settings.auto_clean_enabled {
                        match monoclip_lib::db::queries::run_auto_cleanup(&conn, settings.auto_clean_days) {
                            Ok(count) if count > 0 => log::info!("Auto-cleanup removed {} clips", count),
                            Err(e) => log::error!("Auto-cleanup failed: {}", e),
                            _ => {}
                        }
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Folders
            folders::get_folders,
            folders::create_folder,
            folders::update_folder,
            folders::delete_folder,
            folders::reorder_folders,
            // Clips
            clips::get_clips,
            clips::get_clip,
            clips::pin_clip,
            clips::unpin_clip,
            clips::delete_clip,
            clips::restore_clip,
            clips::hard_delete_clip,
            clips::move_clip,
            clips::copy_to_clipboard,
            clips::save_current_clipboard_to_folder,
            clips::clear_all_clips,
            clips::clear_folder_clips,
            clips::export_folder_clips,
            // Settings
            settings::get_settings,
            settings::update_settings,
            // Utility
            utility::get_stats,
            utility::run_auto_cleanup,
            utility::show_main_window,
            utility::hide_main_window,
            utility::toggle_main_window,
            utility::install_cli,
            utility::check_accessibility,
            utility::open_accessibility_settings,
            utility::do_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MonoClip")
}
