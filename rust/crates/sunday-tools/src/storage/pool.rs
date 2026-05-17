use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use sunday_core::SUNDAYError;

static SQLITE_POOLS: Lazy<Mutex<HashMap<PathBuf, Pool<SqliteConnectionManager>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Returns an r2d2 connection pool for the given SQLite database path.
/// Creates a new pool if one doesn't exist for the path.
pub fn get_sqlite_pool(db_path: &Path) -> Result<Pool<SqliteConnectionManager>, SUNDAYError> {
    let mut pools = SQLITE_POOLS.lock().unwrap();

    if let Some(pool) = pools.get(db_path) {
        return Ok(pool.clone());
    }

    // Special case for in-memory databases - create a unique shared memory connection
    let manager = if db_path.to_string_lossy() == ":memory:" || db_path.to_string_lossy() == "" {
        SqliteConnectionManager::memory()
    } else {
        SqliteConnectionManager::file(db_path)
    };

    let pool = Pool::builder()
        .max_size(10) // Cache up to 10 connections per DB
        .build(manager)
        .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

    pools.insert(db_path.to_path_buf(), pool.clone());
    
    Ok(pool)
}
