pub const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS folders (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    icon            TEXT    NOT NULL DEFAULT '📋',
    color           TEXT    NOT NULL DEFAULT '#6366f1',
    global_shortcut TEXT,
    position        INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO folders (id, name, icon, color, position)
VALUES (1, 'Inbox', '📋', '#6366f1', 0);

CREATE TABLE IF NOT EXISTS clip_items (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    content      TEXT    NOT NULL,
    content_type TEXT    NOT NULL DEFAULT 'text',
    preview      TEXT    NOT NULL DEFAULT '',
    folder_id    INTEGER NOT NULL DEFAULT 1,
    is_pinned    INTEGER NOT NULL DEFAULT 0,
    is_deleted   INTEGER NOT NULL DEFAULT 0,
    source_app   TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE SET DEFAULT
);

CREATE INDEX IF NOT EXISTS idx_clips_folder ON clip_items(folder_id);
CREATE INDEX IF NOT EXISTS idx_clips_deleted_created ON clip_items(is_deleted, created_at);

CREATE TABLE IF NOT EXISTS settings (
    id                  INTEGER PRIMARY KEY DEFAULT 1,
    master_shortcut     TEXT    NOT NULL DEFAULT 'CmdOrCtrl+Shift+V',
    auto_clean_enabled  INTEGER NOT NULL DEFAULT 1,
    auto_clean_days     INTEGER NOT NULL DEFAULT 30,
    max_history_items   INTEGER NOT NULL DEFAULT 500,
    paste_on_click      INTEGER NOT NULL DEFAULT 1,
    theme               TEXT    NOT NULL DEFAULT 'system',
    launch_at_login     INTEGER NOT NULL DEFAULT 1,
    ignored_apps        TEXT    NOT NULL DEFAULT '[]',
    updated_at          TEXT    NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO settings (id) VALUES (1);

CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);

INSERT OR IGNORE INTO schema_version (version) VALUES (1);
"#;

pub fn run_migrations(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    let version: i64 = conn
        .query_row(
            "SELECT version FROM schema_version LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if version < 1 {
        conn.execute_batch(MIGRATION_001)?;
        log::info!("Applied migration 001");
    }

    Ok(())
}
