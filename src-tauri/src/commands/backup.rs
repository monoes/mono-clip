use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::queries;
use crate::state::AppState;

#[derive(Serialize)]
struct ExportFolder {
    name: String,
    icon: String,
    color: String,
    global_shortcut: Option<String>,
    position: i64,
}

#[derive(Serialize)]
struct ExportClip {
    folder_name: String,
    content: String,
    content_type: String,
    preview: String,
    is_pinned: bool,
    created_at: String,
    image_data: Option<String>,
}

#[derive(Serialize)]
struct ExportPayload {
    format_version: u32,
    exported_at: String,
    folders: Vec<ExportFolder>,
    clips: Vec<ExportClip>,
}

#[derive(Deserialize)]
struct ImportClip {
    folder_name: String,
    content: String,
    content_type: String,
    preview: String,
    is_pinned: bool,
    #[allow(dead_code)]
    created_at: String,
    image_data: Option<String>,
}

#[derive(Deserialize)]
struct ImportFolder {
    name: String,
    icon: String,
    color: String,
    global_shortcut: Option<String>,
    #[allow(dead_code)]
    position: i64,
}

#[derive(Deserialize)]
struct ImportPayload {
    #[allow(dead_code)]
    format_version: u32,
    folders: Vec<ImportFolder>,
    clips: Vec<ImportClip>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    folders_created: i64,
    folders_merged: i64,
    clips_imported: i64,
}

#[tauri::command]
pub fn export_data(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock();
    let folders = queries::get_folders(&conn).map_err(|e| e.to_string())?;
    let folder_names: std::collections::HashMap<i64, String> =
        folders.iter().map(|f| (f.id, f.name.clone())).collect();

    let export_folders: Vec<ExportFolder> = folders
        .iter()
        .map(|f| ExportFolder {
            name: f.name.clone(),
            icon: f.icon.clone(),
            color: f.color.clone(),
            global_shortcut: f.global_shortcut.clone(),
            position: f.position,
        })
        .collect();

    let all_clips = queries::get_clips(&conn, None, None, i64::MAX, 0).map_err(|e| e.to_string())?;
    let mut export_clips = Vec::new();
    for clip in all_clips.into_iter().filter(|c| !c.is_deleted) {
        let image_data = if clip.content_type == "image" {
            std::fs::read(&clip.content).ok().map(|bytes| {
                format!("data:image/png;base64,{}", base64_encode(&bytes))
            })
        } else {
            None
        };
        export_clips.push(ExportClip {
            folder_name: folder_names.get(&clip.folder_id).cloned().unwrap_or_else(|| "Inbox".into()),
            content: clip.content,
            content_type: clip.content_type,
            preview: clip.preview,
            is_pinned: clip.is_pinned,
            created_at: clip.created_at,
            image_data,
        });
    }

    let payload = ExportPayload {
        format_version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        folders: export_folders,
        clips: export_clips,
    };
    serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_data(state: State<AppState>, json: String) -> Result<ImportSummary, String> {
    let payload: ImportPayload = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let conn = state.db.lock();

    let mut folders_created = 0i64;
    let mut folders_merged = 0i64;
    let mut folder_ids: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    let existing = queries::get_folders(&conn).map_err(|e| e.to_string())?;
    for f in &existing {
        folder_ids.insert(f.name.clone(), f.id);
    }

    for f in &payload.folders {
        if folder_ids.contains_key(&f.name) {
            folders_merged += 1;
            continue;
        }
        let created = queries::create_folder(&conn, &f.name, &f.icon, &f.color, f.global_shortcut.as_deref())
            .map_err(|e| e.to_string())?;
        folder_ids.insert(f.name.clone(), created.id);
        folders_created += 1;
    }

    let mut clips_imported = 0i64;
    for c in &payload.clips {
        let folder_id = *folder_ids.get(&c.folder_name).unwrap_or(&1);
        let content = if let (Some(data_uri), "image") = (&c.image_data, c.content_type.as_str()) {
            let b64 = data_uri.split(',').nth(1).unwrap_or("");
            let png_bytes = base64_decode(b64);
            // image_store::save_as_png takes decoded RGBA pixels + dimensions,
            // not a raw PNG byte blob — decode first (the `image` crate is
            // already a project dependency).
            match image::load_from_memory(&png_bytes) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    crate::clipboard::image_store::save_as_png(rgba.as_raw(), w, h)
                        .unwrap_or_else(|_| c.content.clone())
                }
                Err(_) => c.content.clone(),
            }
        } else {
            c.content.clone()
        };
        queries::insert_clip(&conn, &content, &c.content_type, &c.preview, folder_id, None)
            .map_err(|e| e.to_string())?;
        if c.is_pinned {
            if let Ok(clip) = queries::get_clip(&conn, conn.last_insert_rowid()) {
                let _ = queries::set_clip_pinned(&conn, clip.id, true);
            }
        }
        clips_imported += 1;
    }

    Ok(ImportSummary { folders_created, folders_merged, clips_imported })
}

// Minimal base64 — avoids adding a dependency for two one-line operations.
// Replace with the `base64` crate if this project already depends on it
// elsewhere by the time this task is implemented (check Cargo.toml first).
fn base64_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        let _ = write!(out, "{}", CHARS[((n >> 18) & 63) as usize] as char);
        let _ = write!(out, "{}", CHARS[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(n & 63) as usize] as char } else { '=' });
    }
    out
}

fn base64_decode(s: &str) -> Vec<u8> {
    fn val(c: u8) -> u32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=').collect();
    let mut out = Vec::new();
    for chunk in clean.chunks(4) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, &c)| acc | (val(c) << (18 - 6 * i)));
        out.push((n >> 16) as u8);
        if chunk.len() > 2 { out.push((n >> 8) as u8); }
        if chunk.len() > 3 { out.push(n as u8); }
    }
    out
}
