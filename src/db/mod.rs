//  SQLite database interactions
pub struct Metadata;

pub fn initialize_database() -> rusqlite::Result<rusqlite::Connection>{
    // Initialize SQLite database connection
    todo!()
}

#[allow(unused_variables)]
pub fn store_room_info(conn: &rusqlite::Connection,room_name: &str,
    creator_device_id: &str, group_names: &str) -> rusqlite::Result<()> {
    // Store room information
    todo!()
}

#[allow(unused_variables)]
pub fn get_room_info(conn: &rusqlite::Connection,
    room_name: &str) -> rusqlite::Result<(String, String)> {
    //Logic to retrieve room information
    todo!()
}

#[allow(unused_variables)]
pub fn store_user_permissions(conn: &rusqlite::Connection, user_device_id: &str,
    permissions: &str) -> rusqlite::Result<()> {
    // Logic to store user permissions
    todo!()
}

#[allow(unused_variables)]
pub fn get_user_permissions(conn: &rusqlite::Connection,
    user_device_id: &str) -> rusqlite::Result<String> {
    // Logic to get user permissions
    todo!()
}

