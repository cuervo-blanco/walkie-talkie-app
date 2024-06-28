use rusqlite::{params, Connection, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
//  SQLite database interactions

pub type SqlitePool = Pool<SqliteConnectionManager>;
pub type SqlitePooledConnection = PooledConnection<SqliteConnectionManager>;

pub fn initialize_pool(db_path: &str) -> SqlitePool {
    let manager = SqliteConnectionManager::file(db_path);
    Pool::new(manager).expect("Failed to create pool.")
}

pub fn initialize_database(pool: &SqlitePool) {
    let conn = pool.get().expect("Failed to get connection from pool");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS rooms (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            metadata TEXT
        )",
        [],
    ).expect("Failed to create rooms table.");
    conn.execute(
        "CREATE TABLE IF NOT user_permissions (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            permissions TEXT
        )",
        [],
    ).expect("Failed to create user_permissions table");
}

pub fn store_room_info(pool: &SqlitePool, name: &str, metadata: &str) {
    // Store room information
    let conn = pool.get().expect("Failed to get connection from pool");
    conn.execute(
        "INSERT INTO rooms (name, metadata) VALUES (?1, ?2)",
        params![name, metadata],
    ).expect("Failed to insert room information");
}

pub fn get_room_info(pool: &SqlitePool, name: &str) -> Result<Option<(i32, String)>> {
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare("SELECT id, metadata FROM rooms WHERE name = ?1")?;
    let room_iter = stmt.query_map(params![name], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).expect("Failed to map query");
    for room in room_iter {
        return Ok(Some(room.expect("Failed to get room info.")));
    }
    Ok(None)
}

pub fn store_user_permissions(pool: &SqlitePool, user: &str,
    permissions: &str) -> rusqlite::Result<()> {
    let conn = pool.get().expect("Failed to get connection from pool");
    conn.execute(
        "INSERT INTO user_permissions (username, permissions) VALUES (?1, ?2)",
        params![user, permissions]
    )?;
    Ok(())
}

pub fn get_user_permissions(pool: &SqlitePool,
    user: &str) -> rusqlite::Result<Option<(i32, String)>> {
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare("SELECT id, permissions FROM user_permissions WHERE username = ?1")?;
    let user_iter = stmt.query_map(params![user], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).expect("Failed to map query.");
    for user in user_iter {
        return Ok(Some(user.expect("Failed to get user permissions.")));
    }
    Ok(None)
}

