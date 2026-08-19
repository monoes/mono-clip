use clap::{Parser, Subcommand};
use rusqlite::{Connection, params};
use std::path::PathBuf;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "mclip",
    about = "MonoClip CLI — manage your clipboard history from the terminal",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List clipboard items
    List {
        /// Show items in a specific folder (default: Inbox)
        #[arg(short, long)]
        folder: Option<String>,
        /// Filter by content
        #[arg(short, long)]
        search: Option<String>,
        /// Maximum number of items to show (default: 20)
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },
    /// Add a new clipboard item
    Add {
        /// The text content to add
        content: String,
        /// Destination folder name (default: Inbox)
        #[arg(short, long)]
        folder: Option<String>,
    },
    /// Remove a clipboard item by ID
    Remove {
        /// Item ID (from `mclip list`)
        id: i64,
    },
    /// Pin a clipboard item
    Pin {
        id: i64,
    },
    /// Unpin a clipboard item
    Unpin {
        id: i64,
    },
    /// Print the raw content of an item (pipe-friendly)
    Get {
        id: i64,
    },
    /// Manage folders
    Folder {
        #[command(subcommand)]
        action: FolderCmd,
    },
    /// Print an AI context block you can paste into any chat or CLAUDE.md
    Context,
    /// Start an MCP stdio server (for Claude Desktop, Cursor, etc.)
    Mcp,
}

#[derive(Subcommand)]
enum FolderCmd {
    /// List all folders
    List,
    /// Create a new folder
    Add {
        name: String,
        #[arg(short, long, default_value = "📋")]
        icon: String,
        #[arg(short, long, default_value = "#6366f1")]
        color: String,
    },
    /// Delete a folder (items are moved to Inbox)
    Remove {
        name: String,
    },
}

// ── Database helpers ──────────────────────────────────────────────────────────

fn db_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .unwrap_or_else(|| std::ffi::OsString::from("."));
    PathBuf::from(home).join(".monoclip").join("monoclip.db")
}

fn open_db() -> Connection {
    let path = db_path();
    if !path.exists() {
        eprintln!("error: MonoClip database not found at {}", path.display());
        eprintln!("       Launch MonoClip.app at least once to create it.");
        std::process::exit(1);
    }
    let conn = Connection::open(&path).unwrap_or_else(|e| {
        eprintln!("error: failed to open database: {}", e);
        std::process::exit(1);
    });
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;",
    )
    .ok();
    conn
}

fn resolve_folder_id(conn: &Connection, name: &str) -> i64 {
    if name.eq_ignore_ascii_case("inbox") {
        return 1;
    }
    let result = conn.query_row(
        "SELECT id FROM folders WHERE LOWER(name) = LOWER(?1) LIMIT 1",
        params![name],
        |r| r.get::<_, i64>(0),
    );
    match result {
        Ok(id) => id,
        Err(_) => {
            eprintln!("error: folder '{}' not found", name);
            std::process::exit(1);
        }
    }
}

fn detect_type(content: &str) -> &'static str {
    let t = content.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        return "url";
    }
    if t.contains('@') && t.contains('.') && !t.contains(' ') {
        return "email";
    }
    if (t.starts_with('#') && (t.len() == 4 || t.len() == 7))
        && t[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return "color";
    }
    let lines: Vec<&str> = t.lines().collect();
    if lines.len() > 1 {
        let code_chars = t.chars().filter(|c| matches!(c, '{' | '}' | ';' | '(' | ')')).count();
        if code_chars > 3 {
            return "code";
        }
    }
    "text"
}

fn make_preview(content: &str, max: usize) -> String {
    let t = content.trim();
    if t.len() <= max {
        t.replace('\n', " ")
    } else {
        format!("{}…", &t[..max].replace('\n', " "))
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}

fn relative_time(updated_at: &str) -> String {
    // updated_at is stored as SQLite datetime (UTC): "2024-01-15 12:34:56"
    let now = chrono::Utc::now().naive_utc();
    let parsed = chrono::NaiveDateTime::parse_from_str(updated_at, "%Y-%m-%d %H:%M:%S")
        .unwrap_or(now);
    let diff = now.signed_duration_since(parsed);
    let secs = diff.num_seconds();
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", diff.num_minutes())
    } else if secs < 86400 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

fn cmd_list(folder: Option<String>, search: Option<String>, limit: i64) {
    let conn = open_db();
    let folder_id = folder.as_deref().map(|f| resolve_folder_id(&conn, f));

    let where_folder = match folder_id {
        Some(id) => format!("AND folder_id = {}", id),
        None => String::new(),
    };
    let where_search = if search.is_some() {
        "AND (content LIKE ?1 OR preview LIKE ?1)"
    } else {
        ""
    };

    let sql = format!(
        "SELECT id, content_type, is_pinned, updated_at, preview, content
         FROM clip_items
         WHERE is_deleted = 0 {where_folder} {where_search}
         ORDER BY is_pinned DESC, updated_at DESC
         LIMIT {limit}"
    );

    println!("{:>5}  {:<6}  {:<4}  {:<12}  {}", "ID", "TYPE", "PIN", "UPDATED", "PREVIEW");
    println!("{}", "─".repeat(70));

    type Row = (i64, String, bool, String, String, String);

    let rows: Vec<Row> = if let Some(q) = search.as_deref() {
        let pattern = format!("%{}%", q);
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map(params![pattern], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? != 0,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .unwrap()
        .flatten()
        .collect()
    } else {
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? != 0,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .unwrap()
        .flatten()
        .collect()
    };

    if rows.is_empty() {
        println!("  (no items)");
        return;
    }
    for (id, ctype, pinned, updated_at, preview, content) in rows {
        let display = if !preview.is_empty() { preview } else { content };
        println!(
            "{:>5}  {:<6}  {:<4}  {:<12}  {}",
            id,
            truncate(&ctype, 6),
            if pinned { "📌" } else { "  " },
            truncate(&relative_time(&updated_at), 12),
            truncate(&display.replace('\n', " "), 46),
        );
    }
}

fn cmd_add(content: String, folder: Option<String>) {
    let conn = open_db();
    let folder_id = folder.as_deref().map(|f| resolve_folder_id(&conn, f)).unwrap_or(1);
    let ctype = detect_type(&content);
    let preview = make_preview(&content, 200);

    conn.execute(
        "INSERT INTO clip_items (content, content_type, preview, folder_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![content, ctype, preview, folder_id],
    ).unwrap_or_else(|e| { eprintln!("error: {}", e); std::process::exit(1); });

    let id = conn.last_insert_rowid();
    let folder_name = folder.as_deref().unwrap_or("Inbox");
    println!("✓ Added clip #{} to {} ({})", id, folder_name, ctype);
}

fn cmd_remove(id: i64) {
    let conn = open_db();
    let affected = conn.execute(
        "UPDATE clip_items SET is_deleted = 1, updated_at = datetime('now') WHERE id = ?1 AND is_deleted = 0",
        params![id],
    ).unwrap_or(0);
    if affected == 0 {
        eprintln!("error: clip #{} not found", id);
        std::process::exit(1);
    }
    println!("✓ Removed clip #{}", id);
}

fn cmd_pin(id: i64, pin: bool) {
    let conn = open_db();
    let affected = conn.execute(
        "UPDATE clip_items SET is_pinned = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![pin as i64, id],
    ).unwrap_or(0);
    if affected == 0 {
        eprintln!("error: clip #{} not found", id);
        std::process::exit(1);
    }
    println!("✓ {} clip #{}", if pin { "Pinned" } else { "Unpinned" }, id);
}

fn cmd_get(id: i64) {
    let conn = open_db();
    let result = conn.query_row(
        "SELECT content FROM clip_items WHERE id = ?1 AND is_deleted = 0",
        params![id],
        |r| r.get::<_, String>(0),
    );
    match result {
        Ok(content) => print!("{}", content),
        Err(_) => {
            eprintln!("error: clip #{} not found", id);
            std::process::exit(1);
        }
    }
}

fn cmd_folder_list() {
    let conn = open_db();
    let mut stmt = conn.prepare(
        "SELECT f.id, f.icon, f.name, COUNT(c.id) as cnt
         FROM folders f
         LEFT JOIN clip_items c ON c.folder_id = f.id AND c.is_deleted = 0
         GROUP BY f.id ORDER BY f.position ASC, f.id ASC"
    ).unwrap();

    println!("{:>4}  {:<2}  {:<24}  {}", "ID", "", "NAME", "ITEMS");
    println!("{}", "─".repeat(40));
    for row in stmt.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?))
    }).unwrap().flatten() {
        println!("{:>4}  {}  {:<24}  {}", row.0, row.1, truncate(&row.2, 24), row.3);
    }
}

fn cmd_folder_add(name: String, icon: String, color: String) {
    let conn = open_db();
    let max_pos: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), 0) FROM folders", [], |r| r.get(0)
    ).unwrap_or(0);
    conn.execute(
        "INSERT INTO folders (name, icon, color, position) VALUES (?1, ?2, ?3, ?4)",
        params![name, icon, color, max_pos + 1],
    ).unwrap_or_else(|e| { eprintln!("error: {}", e); std::process::exit(1); });
    let id = conn.last_insert_rowid();
    println!("✓ Created folder '{}' (id={})", name, id);
}

fn cmd_folder_remove(name: String) {
    let conn = open_db();
    let id = resolve_folder_id(&conn, &name);
    if id == 1 {
        eprintln!("error: cannot delete the Inbox folder");
        std::process::exit(1);
    }
    conn.execute(
        "UPDATE clip_items SET folder_id = 1, updated_at = datetime('now') WHERE folder_id = ?1",
        params![id],
    ).ok();
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id]).ok();
    println!("✓ Deleted folder '{}', items moved to Inbox", name);
}

fn cmd_context() {
    print!("{}", r##"## mclip — MonoClip CLI

`mclip` is a command-line tool that reads and writes to the MonoClip clipboard
manager database on macOS. Use it to list, add, remove, pin, or organise clips
and folders without touching the GUI.

### Clip commands

| Command | Description |
|---------|-------------|
| `mclip list` | List recent Inbox items (20 by default) |
| `mclip list --folder Work` | List items in the "Work" folder |
| `mclip list --search <query>` | Search across all clips |
| `mclip list --limit 50` | Show up to 50 items |
| `mclip add "text"` | Add a text clip to Inbox |
| `mclip add "text" --folder Work` | Add to a specific folder |
| `mclip get <id>` | Print raw content of a clip (pipe-friendly) |
| `mclip remove <id>` | Soft-delete a clip |
| `mclip pin <id>` | Pin a clip (keeps it from auto-cleanup) |
| `mclip unpin <id>` | Unpin a clip |

### Folder commands

| Command | Description |
|---------|-------------|
| `mclip folder list` | List all folders with item counts |
| `mclip folder add "Name"` | Create a new folder |
| `mclip folder add "Name" --icon 🔖 --color "#ff6b6b"` | With custom icon/colour |
| `mclip folder remove "Name"` | Delete a folder (items move to Inbox) |

### Notes
- IDs come from `mclip list` output.
- Folder names are case-insensitive.
- `mclip get <id>` outputs raw text with no newline — ideal for piping:
  `mclip get 42 | pbcopy`
- Pinned clips are never removed by auto-cleanup.
- Images and file paths are stored but `mclip get` returns their stored path string.

### Examples
```bash
# Save the current clipboard content as a named snippet
pbpaste | xargs -I{} mclip add "{}" --folder Snippets

# Find recent URLs
mclip list --search http

# Show what's in the Work folder
mclip list --folder Work --limit 50

# Pipe a clip back to the clipboard
mclip get 7 | pbcopy
```
"##);
}

// ── MCP server ────────────────────────────────────────────────────────────────

fn cmd_mcp() {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();

    // Tool schemas returned to the MCP client
    let tools = serde_json::json!([
        {
            "name": "list_clips",
            "description": "List clipboard items from MonoClip",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "folder": { "type": "string", "description": "Folder name (default: Inbox)" },
                    "search": { "type": "string", "description": "Filter by content" },
                    "limit": { "type": "integer", "description": "Max results (default: 20)" }
                }
            }
        },
        {
            "name": "add_clip",
            "description": "Add a text clip to MonoClip",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Text to save" },
                    "folder": { "type": "string", "description": "Destination folder (default: Inbox)" }
                },
                "required": ["content"]
            }
        },
        {
            "name": "get_clip",
            "description": "Get the raw content of a clip by ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Clip ID (from list_clips)" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "remove_clip",
            "description": "Delete a clip from MonoClip",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Clip ID (from list_clips)" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "pin_clip",
            "description": "Pin or unpin a clip to protect it from auto-cleanup",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Clip ID" },
                    "pin": { "type": "boolean", "description": "true to pin, false to unpin (default: true)" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "list_folders",
            "description": "List all MonoClip folders",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "create_folder",
            "description": "Create a new folder in MonoClip",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Folder name" },
                    "icon": { "type": "string", "description": "Emoji icon (default: 📋)" },
                    "color": { "type": "string", "description": "Hex color (default: #6366f1)" }
                },
                "required": ["name"]
            }
        },
        {
            "name": "delete_folder",
            "description": "Delete a MonoClip folder (items move to Inbox)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Folder name" }
                },
                "required": ["name"]
            }
        }
    ]);

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        let req: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let params = req.get("params").cloned().unwrap_or(serde_json::json!({}));

        // Notifications have no id — skip sending a response
        if id.is_null() && method.starts_with("notifications/") {
            continue;
        }

        let response: serde_json::Value = match method {
            "initialize" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "mclip", "version": env!("CARGO_PKG_VERSION") }
                }
            }),

            "tools/list" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tools }
            }),

            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                let result = mcp_call_tool(tool_name, &args);
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": result
                })
            }

            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            }),
        };

        let mut out = stdout.lock();
        let _ = writeln!(out, "{}", serde_json::to_string(&response).unwrap_or_default());
        let _ = out.flush();
    }
}

fn mcp_call_tool(name: &str, args: &serde_json::Value) -> serde_json::Value {
    let text = |s: String| serde_json::json!({
        "content": [{ "type": "text", "text": s }]
    });
    let err = |s: String| serde_json::json!({
        "content": [{ "type": "text", "text": s }],
        "isError": true
    });

    match name {
        "list_clips" => {
            let folder = args.get("folder").and_then(|v| v.as_str()).map(String::from);
            let search = args.get("search").and_then(|v| v.as_str()).map(String::from);
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
            text(mcp_list_clips(folder, search, limit))
        }
        "add_clip" => {
            let content = match args.get("content").and_then(|v| v.as_str()) {
                Some(c) => c.to_string(),
                None => return err("missing required field: content".to_string()),
            };
            let folder = args.get("folder").and_then(|v| v.as_str()).map(String::from);
            match mcp_add_clip(content, folder) {
                Ok(msg) => text(msg),
                Err(e) => err(e),
            }
        }
        "get_clip" => {
            let id = match args.get("id").and_then(|v| v.as_i64()) {
                Some(i) => i,
                None => return err("missing required field: id".to_string()),
            };
            match mcp_get_clip(id) {
                Ok(content) => text(content),
                Err(e) => err(e),
            }
        }
        "remove_clip" => {
            let id = match args.get("id").and_then(|v| v.as_i64()) {
                Some(i) => i,
                None => return err("missing required field: id".to_string()),
            };
            match mcp_remove_clip(id) {
                Ok(msg) => text(msg),
                Err(e) => err(e),
            }
        }
        "pin_clip" => {
            let id = match args.get("id").and_then(|v| v.as_i64()) {
                Some(i) => i,
                None => return err("missing required field: id".to_string()),
            };
            let pin = args.get("pin").and_then(|v| v.as_bool()).unwrap_or(true);
            match mcp_pin_clip(id, pin) {
                Ok(msg) => text(msg),
                Err(e) => err(e),
            }
        }
        "list_folders" => text(mcp_list_folders()),
        "create_folder" => {
            let name = match args.get("name").and_then(|v| v.as_str()) {
                Some(n) => n.to_string(),
                None => return err("missing required field: name".to_string()),
            };
            let icon = args.get("icon").and_then(|v| v.as_str()).unwrap_or("📋").to_string();
            let color = args.get("color").and_then(|v| v.as_str()).unwrap_or("#6366f1").to_string();
            match mcp_create_folder(name, icon, color) {
                Ok(msg) => text(msg),
                Err(e) => err(e),
            }
        }
        "delete_folder" => {
            let name = match args.get("name").and_then(|v| v.as_str()) {
                Some(n) => n.to_string(),
                None => return err("missing required field: name".to_string()),
            };
            match mcp_delete_folder(name) {
                Ok(msg) => text(msg),
                Err(e) => err(e),
            }
        }
        _ => err(format!("unknown tool: {}", name)),
    }
}

fn mcp_list_clips(folder: Option<String>, search: Option<String>, limit: i64) -> String {
    let conn = open_db();
    let folder_id = folder.as_deref().map(|f| {
        if f.eq_ignore_ascii_case("inbox") { 1 }
        else {
            conn.query_row(
                "SELECT id FROM folders WHERE LOWER(name) = LOWER(?1) LIMIT 1",
                params![f], |r| r.get::<_, i64>(0),
            ).unwrap_or(1)
        }
    });
    let where_folder = folder_id.map(|id| format!("AND folder_id = {}", id)).unwrap_or_default();
    let where_search = if search.is_some() { "AND (content LIKE ?1 OR preview LIKE ?1)" } else { "" };
    let sql = format!(
        "SELECT id, content_type, is_pinned, updated_at, preview, content
         FROM clip_items WHERE is_deleted = 0 {where_folder} {where_search}
         ORDER BY is_pinned DESC, updated_at DESC LIMIT {limit}"
    );
    type Row = (i64, String, bool, String, String, String);
    let rows: Vec<Row> = if let Some(q) = search.as_deref() {
        let pattern = format!("%{}%", q);
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map(params![pattern], |r| Ok((
            r.get::<_, i64>(0)?, r.get::<_, String>(1)?,
            r.get::<_, i64>(2)? != 0, r.get::<_, String>(3)?,
            r.get::<_, String>(4)?, r.get::<_, String>(5)?,
        ))).unwrap().flatten().collect()
    } else {
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], |r| Ok((
            r.get::<_, i64>(0)?, r.get::<_, String>(1)?,
            r.get::<_, i64>(2)? != 0, r.get::<_, String>(3)?,
            r.get::<_, String>(4)?, r.get::<_, String>(5)?,
        ))).unwrap().flatten().collect()
    };
    if rows.is_empty() { return "(no items)".to_string(); }
    let mut out = format!("{:>5}  {:<6}  {:<4}  {:<12}  {}\n{}\n", "ID", "TYPE", "PIN", "UPDATED", "PREVIEW", "─".repeat(70));
    for (id, ctype, pinned, updated_at, preview, content) in rows {
        let display = if !preview.is_empty() { preview } else { content };
        out.push_str(&format!(
            "{:>5}  {:<6}  {:<4}  {:<12}  {}\n",
            id, truncate(&ctype, 6),
            if pinned { "📌" } else { "  " },
            truncate(&relative_time(&updated_at), 12),
            truncate(&display.replace('\n', " "), 46),
        ));
    }
    out
}

fn mcp_add_clip(content: String, folder: Option<String>) -> Result<String, String> {
    let conn = open_db();
    let folder_id = folder.as_deref().map(|f| resolve_folder_id(&conn, f)).unwrap_or(1);
    let ctype = detect_type(&content);
    let preview = make_preview(&content, 200);
    conn.execute(
        "INSERT INTO clip_items (content, content_type, preview, folder_id) VALUES (?1, ?2, ?3, ?4)",
        params![content, ctype, preview, folder_id],
    ).map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(format!("Added clip #{} to {} ({})", id, folder.as_deref().unwrap_or("Inbox"), ctype))
}

fn mcp_get_clip(id: i64) -> Result<String, String> {
    let conn = open_db();
    conn.query_row(
        "SELECT content FROM clip_items WHERE id = ?1 AND is_deleted = 0",
        params![id], |r| r.get::<_, String>(0),
    ).map_err(|_| format!("clip #{} not found", id))
}

fn mcp_remove_clip(id: i64) -> Result<String, String> {
    let conn = open_db();
    let affected = conn.execute(
        "UPDATE clip_items SET is_deleted = 1, updated_at = datetime('now') WHERE id = ?1 AND is_deleted = 0",
        params![id],
    ).unwrap_or(0);
    if affected == 0 { return Err(format!("clip #{} not found", id)); }
    Ok(format!("Removed clip #{}", id))
}

fn mcp_pin_clip(id: i64, pin: bool) -> Result<String, String> {
    let conn = open_db();
    let affected = conn.execute(
        "UPDATE clip_items SET is_pinned = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![pin as i64, id],
    ).unwrap_or(0);
    if affected == 0 { return Err(format!("clip #{} not found", id)); }
    Ok(format!("{} clip #{}", if pin { "Pinned" } else { "Unpinned" }, id))
}

fn mcp_list_folders() -> String {
    let conn = open_db();
    let mut stmt = conn.prepare(
        "SELECT f.id, f.icon, f.name, COUNT(c.id) as cnt
         FROM folders f LEFT JOIN clip_items c ON c.folder_id = f.id AND c.is_deleted = 0
         GROUP BY f.id ORDER BY f.position ASC, f.id ASC"
    ).unwrap();
    let mut out = format!("{:>4}  {:<2}  {:<24}  {}\n{}\n", "ID", "", "NAME", "ITEMS", "─".repeat(40));
    for row in stmt.query_map([], |r| Ok((
        r.get::<_, i64>(0)?, r.get::<_, String>(1)?,
        r.get::<_, String>(2)?, r.get::<_, i64>(3)?,
    ))).unwrap().flatten() {
        out.push_str(&format!("{:>4}  {}  {:<24}  {}\n", row.0, row.1, truncate(&row.2, 24), row.3));
    }
    out
}

fn mcp_create_folder(name: String, icon: String, color: String) -> Result<String, String> {
    let conn = open_db();
    let max_pos: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), 0) FROM folders", [], |r| r.get(0)
    ).unwrap_or(0);
    conn.execute(
        "INSERT INTO folders (name, icon, color, position) VALUES (?1, ?2, ?3, ?4)",
        params![name, icon, color, max_pos + 1],
    ).map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(format!("Created folder '{}' (id={})", name, id))
}

fn mcp_delete_folder(name: String) -> Result<String, String> {
    let conn = open_db();
    let id = conn.query_row(
        "SELECT id FROM folders WHERE LOWER(name) = LOWER(?1) LIMIT 1",
        params![name], |r| r.get::<_, i64>(0),
    ).map_err(|_| format!("folder '{}' not found", name))?;
    if id == 1 { return Err("cannot delete the Inbox folder".to_string()); }
    conn.execute(
        "UPDATE clip_items SET folder_id = 1, updated_at = datetime('now') WHERE folder_id = ?1",
        params![id],
    ).ok();
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id]).ok();
    Ok(format!("Deleted folder '{}', items moved to Inbox", name))
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    monoclip_lib::crash_report::install("mclip");
    let cli = Cli::parse();
    match cli.command {
        Cmd::List { folder, search, limit } => cmd_list(folder, search, limit),
        Cmd::Add { content, folder } => cmd_add(content, folder),
        Cmd::Remove { id } => cmd_remove(id),
        Cmd::Pin { id } => cmd_pin(id, true),
        Cmd::Unpin { id } => cmd_pin(id, false),
        Cmd::Get { id } => cmd_get(id),
        Cmd::Folder { action } => match action {
            FolderCmd::List => cmd_folder_list(),
            FolderCmd::Add { name, icon, color } => cmd_folder_add(name, icon, color),
            FolderCmd::Remove { name } => cmd_folder_remove(name),
        },
        Cmd::Context => cmd_context(),
        Cmd::Mcp => cmd_mcp(),
    }
}
