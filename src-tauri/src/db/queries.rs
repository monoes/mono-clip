use rusqlite::{Connection, params};
use anyhow::Result;
use crate::db::models::{ClipItem, Folder, Settings, AppStats};

// ─── Folder Queries ───────────────────────────────────────────────────────────

pub fn get_folders(conn: &Connection) -> Result<Vec<Folder>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, icon, color, global_shortcut, position, created_at, updated_at
         FROM folders ORDER BY position ASC, id ASC"
    )?;
    let items = stmt.query_map([], |row| {
        Ok(Folder {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            color: row.get(3)?,
            global_shortcut: row.get(4)?,
            position: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(items)
}

pub fn get_folder(conn: &Connection, id: i64) -> Result<Folder> {
    let folder = conn.query_row(
        "SELECT id, name, icon, color, global_shortcut, position, created_at, updated_at
         FROM folders WHERE id = ?1",
        params![id],
        |row| Ok(Folder {
            id: row.get(0)?,
            name: row.get(1)?,
            icon: row.get(2)?,
            color: row.get(3)?,
            global_shortcut: row.get(4)?,
            position: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    )?;
    Ok(folder)
}

pub fn create_folder(
    conn: &Connection,
    name: &str,
    icon: &str,
    color: &str,
    shortcut: Option<&str>,
) -> Result<Folder> {
    let max_pos: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), 0) FROM folders",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO folders (name, icon, color, global_shortcut, position)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, icon, color, shortcut, max_pos + 1],
    )?;
    let id = conn.last_insert_rowid();
    get_folder(conn, id)
}

pub fn update_folder(
    conn: &Connection,
    id: i64,
    name: Option<&str>,
    icon: Option<&str>,
    color: Option<&str>,
    shortcut: Option<Option<&str>>,
) -> Result<Folder> {
    if let Some(n) = name {
        conn.execute("UPDATE folders SET name = ?1, updated_at = datetime('now') WHERE id = ?2", params![n, id])?;
    }
    if let Some(i) = icon {
        conn.execute("UPDATE folders SET icon = ?1, updated_at = datetime('now') WHERE id = ?2", params![i, id])?;
    }
    if let Some(c) = color {
        conn.execute("UPDATE folders SET color = ?1, updated_at = datetime('now') WHERE id = ?2", params![c, id])?;
    }
    if let Some(s) = shortcut {
        conn.execute("UPDATE folders SET global_shortcut = ?1, updated_at = datetime('now') WHERE id = ?2", params![s, id])?;
    }
    get_folder(conn, id)
}

pub fn delete_folder(conn: &Connection, id: i64) -> Result<()> {
    // Move all clips to Inbox
    conn.execute(
        "UPDATE clip_items SET folder_id = 1, updated_at = datetime('now') WHERE folder_id = ?1",
        params![id],
    )?;
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn reorder_folders(conn: &Connection, ids: &[i64]) -> Result<()> {
    for (pos, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE folders SET position = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![pos as i64, id],
        )?;
    }
    Ok(())
}

// ─── Clip Queries ─────────────────────────────────────────────────────────────

pub fn get_clips(
    conn: &Connection,
    folder_id: Option<i64>,
    search: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClipItem>> {
    let (sql, folder_filter) = match folder_id {
        Some(id) => (
            format!(
                "SELECT id, content, content_type, preview, folder_id, is_pinned, is_deleted, source_app, created_at, updated_at
                 FROM clip_items
                 WHERE is_deleted = 0 AND folder_id = {id} {}
                 ORDER BY is_pinned DESC, updated_at DESC
                 LIMIT {} OFFSET {}",
                if search.is_some() { "AND content LIKE ?1" } else { "" },
                limit,
                offset
            ),
            true,
        ),
        None => (
            format!(
                "SELECT id, content, content_type, preview, folder_id, is_pinned, is_deleted, source_app, created_at, updated_at
                 FROM clip_items
                 WHERE is_deleted = 0 {}
                 ORDER BY is_pinned DESC, updated_at DESC
                 LIMIT {} OFFSET {}",
                if search.is_some() { "AND content LIKE ?1" } else { "" },
                limit,
                offset
            ),
            false,
        ),
    };
    let _ = folder_filter; // suppress warning

    let mut stmt = conn.prepare(&sql)?;
    let items = if let Some(q) = search {
        let pattern = format!("%{}%", q);
        stmt.query_map(params![pattern], map_clip_row)?
    } else {
        stmt.query_map([], map_clip_row)?
    }
    .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(items)
}

fn map_clip_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipItem> {
    Ok(ClipItem {
        id: row.get(0)?,
        content: row.get(1)?,
        content_type: row.get(2)?,
        preview: row.get(3)?,
        folder_id: row.get(4)?,
        is_pinned: row.get::<_, i64>(5)? != 0,
        is_deleted: row.get::<_, i64>(6)? != 0,
        source_app: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn get_clip(conn: &Connection, id: i64) -> Result<ClipItem> {
    let item = conn.query_row(
        "SELECT id, content, content_type, preview, folder_id, is_pinned, is_deleted, source_app, created_at, updated_at
         FROM clip_items WHERE id = ?1",
        params![id],
        map_clip_row,
    )?;
    Ok(item)
}

pub fn insert_clip(
    conn: &Connection,
    content: &str,
    content_type: &str,
    preview: &str,
    folder_id: i64,
    source_app: Option<&str>,
) -> Result<ClipItem> {
    conn.execute(
        "INSERT INTO clip_items (content, content_type, preview, folder_id, source_app)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![content, content_type, preview, folder_id, source_app],
    )?;
    let id = conn.last_insert_rowid();
    get_clip(conn, id)
}

pub fn find_duplicate_in_folder(conn: &Connection, content: &str, folder_id: i64) -> Result<Option<i64>> {
    let result = conn.query_row(
        "SELECT id FROM clip_items WHERE content = ?1 AND folder_id = ?2 AND is_deleted = 0 LIMIT 1",
        params![content, folder_id],
        |r| r.get::<_, i64>(0),
    );
    match result {
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn touch_clip(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clip_items SET updated_at = datetime('now') WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn set_clip_pinned(conn: &Connection, id: i64, pinned: bool) -> Result<()> {
    conn.execute(
        "UPDATE clip_items SET is_pinned = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![pinned as i64, id],
    )?;
    Ok(())
}

pub fn soft_delete_clip(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clip_items SET is_deleted = 1, updated_at = datetime('now') WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn restore_clip(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clip_items SET is_deleted = 0, updated_at = datetime('now') WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn hard_delete_clip(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM clip_items WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn move_clip(conn: &Connection, id: i64, folder_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clip_items SET folder_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![folder_id, id],
    )?;
    Ok(())
}

pub fn run_auto_cleanup(conn: &Connection, days: i64) -> Result<i64> {
    // 1. Hard-delete clips that are already soft-deleted (already "trashed")
    let purged_trash = conn.execute(
        "DELETE FROM clip_items WHERE is_deleted = 1",
        [],
    ).unwrap_or(0);

    // 2. Hard-delete old non-pinned clips beyond the retention window,
    //    keeping the 10 most recent Inbox items regardless of age
    let purged_old = conn.execute(
        "DELETE FROM clip_items
         WHERE is_pinned = 0
           AND is_deleted = 0
           AND created_at < datetime('now', ?1)
           AND id NOT IN (
             SELECT id FROM clip_items
             WHERE folder_id = 1 AND is_deleted = 0
             ORDER BY updated_at DESC LIMIT 10
           )",
        params![format!("-{} days", days)],
    ).unwrap_or(0);

    Ok((purged_trash + purged_old) as i64)
}

pub fn clear_all_clips(conn: &Connection) -> Result<i64> {
    let count = conn.execute(
        "DELETE FROM clip_items WHERE is_pinned = 0",
        [],
    )?;
    Ok(count as i64)
}

pub fn clear_folder_clips(conn: &Connection, folder_id: i64) -> Result<i64> {
    let count = conn.execute(
        "DELETE FROM clip_items WHERE folder_id = ?1 AND is_pinned = 0",
        params![folder_id],
    )?;
    Ok(count as i64)
}

pub fn export_folder_clips(conn: &Connection, folder_id: i64) -> Result<Vec<ClipItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, content_type, preview, folder_id, is_pinned, is_deleted, source_app, created_at, updated_at
         FROM clip_items
         WHERE folder_id = ?1 AND is_deleted = 0
         ORDER BY is_pinned DESC, updated_at DESC"
    )?;
    let items = stmt.query_map(params![folder_id], map_clip_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(items)
}

// ─── Settings Queries ─────────────────────────────────────────────────────────

pub fn get_settings(conn: &Connection) -> Result<Settings> {
    let s = conn.query_row(
        "SELECT id, master_shortcut, auto_clean_enabled, auto_clean_days, max_history_items,
                paste_on_click, theme, launch_at_login, ignored_apps, updated_at
         FROM settings WHERE id = 1",
        [],
        |row| Ok(Settings {
            id: row.get(0)?,
            master_shortcut: row.get(1)?,
            auto_clean_enabled: row.get::<_, i64>(2)? != 0,
            auto_clean_days: row.get(3)?,
            max_history_items: row.get(4)?,
            paste_on_click: row.get::<_, i64>(5)? != 0,
            theme: row.get(6)?,
            launch_at_login: row.get::<_, i64>(7)? != 0,
            ignored_apps: row.get(8)?,
            updated_at: row.get(9)?,
        }),
    )?;
    Ok(s)
}

pub fn update_settings(conn: &Connection, patch: &crate::db::models::SettingsPatch) -> Result<Settings> {
    if let Some(v) = &patch.master_shortcut {
        // Unlike folder shortcuts, the master shortcut has no valid "unset" state —
        // it's the only keyboard way to open the app. Reject empty values here as a
        // second line of defense (the UI also disables clearing it) rather than
        // silently persisting something Shortcut::from_str will fail to register.
        if v.is_empty() {
            log::warn!("Ignoring attempt to set master_shortcut to an empty value");
        } else {
            conn.execute("UPDATE settings SET master_shortcut = ?1, updated_at = datetime('now') WHERE id = 1", params![v])?;
        }
    }
    if let Some(v) = patch.auto_clean_enabled {
        conn.execute("UPDATE settings SET auto_clean_enabled = ?1, updated_at = datetime('now') WHERE id = 1", params![v as i64])?;
    }
    if let Some(v) = patch.auto_clean_days {
        conn.execute("UPDATE settings SET auto_clean_days = ?1, updated_at = datetime('now') WHERE id = 1", params![v])?;
    }
    if let Some(v) = patch.max_history_items {
        conn.execute("UPDATE settings SET max_history_items = ?1, updated_at = datetime('now') WHERE id = 1", params![v])?;
    }
    if let Some(v) = patch.paste_on_click {
        conn.execute("UPDATE settings SET paste_on_click = ?1, updated_at = datetime('now') WHERE id = 1", params![v as i64])?;
    }
    if let Some(v) = &patch.theme {
        conn.execute("UPDATE settings SET theme = ?1, updated_at = datetime('now') WHERE id = 1", params![v])?;
    }
    if let Some(v) = patch.launch_at_login {
        conn.execute("UPDATE settings SET launch_at_login = ?1, updated_at = datetime('now') WHERE id = 1", params![v as i64])?;
    }
    if let Some(v) = &patch.ignored_apps {
        conn.execute("UPDATE settings SET ignored_apps = ?1, updated_at = datetime('now') WHERE id = 1", params![v])?;
    }
    get_settings(conn)
}

// ─── Stats ────────────────────────────────────────────────────────────────────

pub fn get_stats(conn: &Connection) -> Result<AppStats> {
    let total_clips: i64 = conn.query_row(
        "SELECT COUNT(*) FROM clip_items WHERE is_deleted = 0", [], |r| r.get(0))?;
    let folders_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM folders", [], |r| r.get(0))?;
    let pinned_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM clip_items WHERE is_pinned = 1 AND is_deleted = 0", [], |r| r.get(0))?;
    Ok(AppStats { total_clips, folders_count, pinned_count })
}
