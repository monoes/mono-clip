use std::path::PathBuf;
use rusqlite::Connection;
use parking_lot::Mutex;
use std::sync::Arc;
use anyhow::Result;

pub type DbConn = Arc<Mutex<Connection>>;

pub fn open_database() -> Result<DbConn> {
    let db_dir = get_db_dir()?;
    std::fs::create_dir_all(&db_dir)?;
    let db_path = db_dir.join("monoclip.db");

    let conn = Connection::open(&db_path)?;

    // Performance pragmas
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA temp_store=MEMORY;
         PRAGMA mmap_size=268435456;",
    )?;

    crate::db::migrations::run_migrations(&conn)?;

    log::info!("Database opened at {:?}", db_path);
    Ok(Arc::new(Mutex::new(conn)))
}

fn get_db_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;
    Ok(home.join(".monoclip"))
}
