//! Global shortcut registration via the XDG Desktop Portal `GlobalShortcuts`
//! interface — the correct mechanism on native-Wayland compositors, where the
//! X11-based `tauri-plugin-global-shortcut` grab used elsewhere in this module
//! never receives physical keypresses from native-Wayland client windows.
//!
//! Unlike the X11 path, the compositor — not this app — owns the final key
//! binding: `preferred_trigger` below is only a hint, and the portal may
//! prompt the user to confirm or choose their own combo instead. Rebinding
//! after the fact (e.g. from Settings) isn't wired up here; a changed master
//! shortcut takes effect on the next app restart, which re-runs `bind_shortcuts`
//! with the new preferred trigger.
//!
//! Not every portal backend implements this interface yet — as of writing,
//! xdg-desktop-portal-cosmic (Pop!_OS's COSMIC session) does not. When it's
//! missing, every step below fails cleanly and is logged; the app keeps
//! running normally and the tray icon remains the reliable way to open it.

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::CreateSessionOptions;
use futures_util::StreamExt;
use tauri::AppHandle;

const TOGGLE_WINDOW_ID: &str = "toggle-window";

/// Convert MonoClip's shortcut string format ("CmdOrCtrl+Shift+V") into the
/// XDG shortcuts-spec trigger format ("CTRL+SHIFT+v"): modifiers upper-case,
/// the key itself lower-case, joined by "+".
/// https://specifications.freedesktop.org/shortcuts-spec/latest/
fn to_portal_trigger(shortcut_str: &str) -> String {
    shortcut_str
        .split('+')
        .map(|part| match part {
            "CmdOrCtrl" | "Ctrl" | "Control" => "CTRL".to_string(),
            "Alt" | "Option" => "ALT".to_string(),
            "Shift" => "SHIFT".to_string(),
            "Super" | "Cmd" | "Meta" => "LOGO".to_string(),
            key => key.to_lowercase(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// Bind the master (open/close) shortcut through the portal and, if that
/// succeeds, listen for activations for the lifetime of the app. Every
/// failure is logged and returns early rather than propagating — this always
/// runs as a detached background task, so there's nothing for a caller to do
/// with an error anyway.
pub async fn run(app: AppHandle, shortcut_str: String) {
    let trigger = to_portal_trigger(&shortcut_str);

    let global_shortcuts = match GlobalShortcuts::new().await {
        Ok(gs) => gs,
        Err(e) => {
            log::warn!(
                "GlobalShortcuts portal unavailable ({e}) — the keyboard shortcut to \
                 open MonoClip won't work on this session; use the tray icon instead."
            );
            return;
        }
    };

    let session = match global_shortcuts.create_session(CreateSessionOptions::default()).await {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to create GlobalShortcuts portal session: {e}");
            return;
        }
    };

    let new_shortcut = NewShortcut::new(TOGGLE_WINDOW_ID, "Open / close MonoClip")
        .preferred_trigger(Some(trigger.as_str()));

    let request = match global_shortcuts
        .bind_shortcuts(&session, &[new_shortcut], None, Default::default())
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Failed to request shortcut binding from the portal: {e}");
            return;
        }
    };

    match request.response() {
        Ok(bound) => {
            let triggers: Vec<&str> = bound
                .shortcuts()
                .iter()
                .map(|s| s.trigger_description())
                .collect();
            log::info!("Master shortcut bound via GlobalShortcuts portal: {:?}", triggers);
        }
        Err(e) => {
            log::warn!("Portal declined to bind the master shortcut: {e}");
            return;
        }
    }

    let mut activated = match global_shortcuts.receive_activated().await {
        Ok(stream) => stream,
        Err(e) => {
            log::warn!("Failed to listen for portal shortcut activations: {e}");
            return;
        }
    };

    while let Some(event) = activated.next().await {
        if event.shortcut_id() == TOGGLE_WINDOW_ID {
            crate::shortcuts::manager::toggle_main_window(&app);
        }
    }
}
