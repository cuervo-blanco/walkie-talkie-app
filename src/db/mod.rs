// ============================================
//                  Imports
// ============================================
#[allow(unused_imports)]
use rusqlite::{params, Connection, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::HashMap;
use crate::discovery;

// ============================================
//          Type Definitions
// ============================================
pub type SqlitePool = Pool<SqliteConnectionManager>;
pub type SqlitePooledConnection = PooledConnection<SqliteConnectionManager>;

// ============================================
//        Initialize Connection Pool
// ============================================
pub fn initialize_pool(db_path: &str) -> SqlitePool {
    let manager = SqliteConnectionManager::file(db_path);
    Pool::new(manager).expect("Failed to create pool.")
}
// ============================================
//          Initialize Database
// ============================================
pub fn initialize_database(pool: &SqlitePool) {
    let conn = pool.get().expect("Failed to get connection from pool");

    // Create the rooms table with a metadata field to store JSON data
    conn.execute(
        "CREATE TABLE IF NOT EXISTS rooms (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            port INTEGER NOT NULL,
            creator_device_id TEXT NOT NULL,
            metadata TEXT
        )",
        [],
    ).expect("Failed to create rooms table.");
}
// ============================================
//          Store Room Information
// ============================================
pub fn store_room_info(pool: &SqlitePool, room: &discovery::Room) {

    //---------------------TODO----------------------//
    // ADD A HELPER FUNCITON TO DETERMINE HOW TO UPDATE THE ROOM METADATA.

    // Store room information
    // Convert the room metadata (Hashmap<String, String>) into JSON
    let room_metadata = serde_json::to_string(&room.metadata).unwrap();
    let conn = pool.get().expect("Failed to get connection from pool");
    conn.execute(
        "INSERT INTO rooms (name, address, port, creator_device_id, metadata)
        VALUES (?1, ?2, ?3, ?4, ?5)",
        params![room.name, room.address.to_string(), room.port,
            room.creator_device_id, room_metadata],
    ).expect("Failed to insert room information");
}
// ============================================
//           Get Room Information
// ============================================
pub fn get_room_info(pool: &SqlitePool, name: &str) -> Result<Option<(i32, String)>> {
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare("SELECT * FROM rooms WHERE name = ?1")?;
    let room_iter = stmt.query_map(params![name], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).expect("Failed to map query");
    for room in room_iter {
        return Ok(Some(room.expect("Failed to get room info.")));
    }
    Ok(None)
}
// ============================================
//            Load Rooms
// ============================================
pub fn load_rooms(pool: &SqlitePool) -> Vec<discovery::Room> {
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare(
        "SELECT name, address, port, creator_device_id, metadata FROM rooms"
    ).expect("Failed to prepare statement");

    let room_iter = stmt.query_map([], |row| {
        let metadata: String = row.get(4)?;
        let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata)
            .expect("Failed to deserialize metadata");
        Ok(discovery::Room {
            name: row.get(0)?,
            address: row.get::<_, String>(1)?.parse().expect("Failed to parse IP address"),
            port: row.get(2)?,
            creator_device_id: row.get(3)?,
            metadata
        })
    }).expect("Failed to map query");

    let mut rooms = Vec::new();
    for room in room_iter {
        rooms.push(room.expect("Failed to get room info."));
    }
    rooms
}
pub fn get_room_metadata(
    pool: &SqlitePool, 
    ws_url: &str
    ) -> Result<HashMap<String, serde_json::Value>> {
        let conn = pool.get().expect("Failed to get connection from pool");

        // Extract the room name from the ws_url
        let url_parts: Vec<&str> = ws_url.split("/").collect();
        let room_name = url_parts.last().unwrap_or(&"");

        let mut stmt = conn.prepare("SELECT metadata FROM rooms WHERE name = ?1")?;
        let metadata: Result<String, rusqlite::Error> = stmt.query_row(
            params![room_name], |row| row.get(0));

        match metadata {
            Ok(metadata_str) => {
                let metadata_map: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata_str)
                    .expect("Failed to deserialize metadata");
                Ok(metadata_map)
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(HashMap::new()),
            Err(e) => Err(e),
        }
}
