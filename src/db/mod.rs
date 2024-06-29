// ============================================
//                  Imports
// ============================================
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS rooms (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            port INTEGER NOT NULL,
            creator_device_id TEXT NOT NULL,
            properties TEXT
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
// ============================================
//          Store Peer Connection
// ============================================
pub fn store_peer_connection_info(pool: &SqlitePool, peer_id: &str,
    connection_state: &str, group_memberships: &[String]) {
    let conn = pool.get().expect("Failed to get connection from pool");
    let group_memberships_str = serde_json::to_string(group_memberships)
        .expect("Failed to serialize group memberships");
    conn.execute(
        "INSERT INTO peer_connections (peer_id, connection_state,
        groups_memberships) VALUES (?1, ?2, ?3) ON CONFLICT (peer_id)
        DO UPDATE SET connection_state = ?2, group_memberships = ?3",
        params![peer_id, connection_state, group_memberships_str],
    ).expect("Failed to insert or update peer connection information");
}
// ============================================
//          Store Room Information
// ============================================
pub fn store_room_info(pool: &SqlitePool, room: &discovery::Room) {
    // Store room information
    // Convert the room properties (Hashmap<String, String>) into JSON
    let room_properties = serde_json::to_string(&room.properties).unwrap();
    let conn = pool.get().expect("Failed to get connection from pool");
    conn.execute(
        "INSERT INTO rooms (name, address, port, creator_device_id, properties)
        VALUES (?1, ?2, ?3, ?4, ?5)",
        params![room.name, room.address.to_string(), room.port,
            room.creator_device_id, room_properties],
    ).expect("Failed to insert room information");
}
// ============================================
//           Get Room Information
// ============================================
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
// ============================================
//            Load Peer Connections
// ============================================
pub fn load_peer_connections(pool: &SqlitePool) -> Vec<(String, String, Vec<String>)>{
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare(
        "SELECT peer_id, connection_state, group_memberships FROM peer_connections"
    ).expect("Failed to prepare statement");

    let peer_iter = stmt.query_map([], |row| {
        let group_memberships_str: String = row.get(2)?;
        let group_memberships: Vec<String> = serde_json::from_str(&group_memberships_str)
            .expect("Failed to deserialize group memberships");
        Ok((row.get(0)?, row.get(1)?, group_memberships))
    }).expect("Failed to map query");

    let mut peers = Vec::new();
    for peer in peer_iter {
        peers.push(peer.expect("Failed to get peer info."));
    }
    peers
}
// ============================================
//            Load Rooms
// ============================================
pub fn load_rooms(pool: &SqlitePool) -> Vec<discovery::Room> {
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare(
        "SELECT name, address, port, creator_device_id, properties FROM rooms"
    ).expect("Failed to prepare statement");

    let room_iter = stmt.query_map([], |row| {
        let properties: String = row.get(4)?;
        let properties: HashMap<String, String> = serde_json::from_str(&properties)
            .expect("Failed to deserialize properties");
        Ok(discovery::Room {
            name: row.get(0)?,
            address: row.get(1)?.parse().expect("Failed to parse IP address"),
            port: row.get(2)?,
            creator_device_id: row.get(3)?,
            properties
        })
    }).expect("Failed to map query");

    let mut rooms = Vec::new();
    for room in room_iter {
        rooms.push(room.expect("Failed to get room info."));
    }
    rooms
}
// ============================================
//        Store User Permissions
// ============================================
pub fn store_user_permissions(pool: &SqlitePool, user: &str,
    permissions: &str) -> rusqlite::Result<()> {
    let conn = pool.get().expect("Failed to get connection from pool");
    conn.execute(
        "INSERT INTO user_permissions (username, permissions) VALUES (?1, ?2)",
        params![user, permissions]
    )?;
    Ok(())
}
pub fn store_audio_channel_info(pool: &SqlitePool, group_name: &str, data_channel_info: &str) {
    let conn = pool.get().expect("Failed to get connection from pool");
    conn.execute(
        "INERT INTO audio_channels (group_name, data_channel_info) VALUES (?1, ?2)
        ON CONFLICT(group_name) DO UPDATE SET data_channel_info = ?2",
        params![group_name, data_channel_info]
    ).expect("Failed to insert or update audio channel information");
}
pub fn load_audio_channels(pool: &SqlitePool) -> Vec<(String, String)> {
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare(
        "SELECT group_name, data_channel_info FROM audio_channels"
    ).expect("Failed to prepare statement");

    let channel_iter = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }).expect("Failed to map query");

    let mut channels = Vec::new();
    for channel in channel_iter {
        channels.push(channel.expect("Failed to get audio channel info."));
    }
    channels
}
// ============================================
//         Get User Permissions
// ============================================
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

