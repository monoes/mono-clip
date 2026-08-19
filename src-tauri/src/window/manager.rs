use tauri::{AppHandle, Manager};

pub fn position_window_near_cursor(app: &AppHandle) {
    use tauri::PhysicalPosition;

    let Some(window) = app.get_webview_window("main") else { return };

    let win_size = window.outer_size().unwrap_or(tauri::PhysicalSize { width: 680, height: 520 });
    let win_w = win_size.width as i32;
    let win_h = win_size.height as i32;

    if let Ok(cursor) = app.cursor_position() {
        let monitor = window
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| window.primary_monitor().ok().flatten());
        if let Some(monitor) = monitor {
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let min_x = mon_pos.x;
            let max_x = mon_pos.x + mon_size.width as i32 - win_w;
            let min_y = mon_pos.y;
            let max_y = mon_pos.y + mon_size.height as i32 - win_h;
            let desired_x = cursor.x as i32 - win_w / 2;
            let desired_y = cursor.y as i32 + 16;
            let x = desired_x.max(min_x).min(max_x.max(min_x));
            let y = desired_y.max(min_y).min(max_y.max(min_y));
            let _ = window.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }

    // Center the window on the primary monitor
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let screen_size = monitor.size();
        let x = (screen_size.width as i32 - win_w) / 2;
        let y = (screen_size.height as i32 - win_h) / 3;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}
