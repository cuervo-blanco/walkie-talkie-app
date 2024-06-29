// ============================================
//                  Imports
// ============================================
// mDNS or custom discovery
#[allow(unused_imports)]
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent, ServiceDaemonError, TxtProperties, TxtProperty};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use crate::db;

// ============================================
//                 Structures
// ============================================

pub struct Room {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub creator_device_id: String,
    pub properties: HashMap<String, String> // This can store user permissions, group memberships, etc.
}
// ============================================
//              mDNS Responder
// ============================================
pub fn start_mdns_responder() -> Result<ServiceDaemon, ServiceDaemonError> {
    // Initialize and return an mDNS ServiceDaemon
    ServiceDaemon::new()
}
// ============================================
//            Database Loading
// ============================================
pub fn load_rooms(pool: &db::SqlitePool) -> Vec<Room> {
    let conn = pool.get().expect("Failed to get connection from pool");
    let mut stmt = conn.prepare(
        "SELECT name, address, port, creator_device_id, properties FROM rooms"
    ).expect("Failed to prepare statement");

    let room_iter = stmt.query_map([], |row| {
        let properties: String = row.get(4)?;
        let properties: HashMap<String, String> = serde_json::from_str(&properties)
            .expect("Failed to deserialize properties");
        Ok(Room {
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
//            Broadcast Service
// ============================================
pub fn broadcast_service(
    pool: &db::SqlitePool, room_name: &str, creator_device_id: &str, port: u16
) -> Result<ServiceInfo, Box<dyn std::error::Error>> {
    // Logic to broadcast mDNS service
    let service_name = format!("{}_{}", room_name, creator_device_id);
    let service_type = "_http._tcp.local"; // Service type for mDNS
    let hostname = format!("{}.local.", creator_device_id);
    let instance_name = room_name.to_string();
    let properties = HashMap::new();

    let service_info = ServiceInfo::new(
        &service_type,
        &service_name,
        &hostname,
        &instance_name,
        port,
        properties,
    )?;

    let responder = start_mdns_responder()?;
    responder.register(service_info.clone())?;

    let room = Room {
        name: room_name.to_string(),
        address: IpAddr::V4(Ipv4Addr::LOCALHOST), //Replace with actual address
        port,
        creator_device_id: creator_device_id.to_string(),
        properties: HashMap::new(), // Add actual properties
    };
    db::store_room_info(pool, &room);

    Ok(service_info)
}

pub fn load_and_broadcast_services(pool: &db::SqlitePool)
-> Result<(), Box<dyn std::error::Error>> {
    let rooms = load_rooms(pool);
    let responder = start_mdns_responder()?;

    for room in rooms {
        let service_name = format!("{}_{}", room.name, room.creator_device_id);
        let service_type = "_http._tcp.local"; // Service type for mDNS
        let hostname = format!("{}.local.", room.creator_device_id);
        let instance_name = room.name.clone();
        let properties = HashMap::new(); // Add actual properties

    let service_info = ServiceInfo::new(
        &service_type,
        &service_name,
        &hostname,
        &instance_name,
        room.port,
        properties,
    )?;

    responder.register(service_info.clone())?;
    }
    Ok(())
}
// ============================================
//            Discover Networks
// ============================================
pub fn discover_networks(service_type: &str) -> Result<Receiver<mdns_sd::ServiceEvent>, Box<dyn std::error::Error>> {
    let responder = start_mdns_responder()?;
    let (sender, receiver) = mpsc::channel();

    let _service_discovery = responder.browse(service_type, move |result| {
            match result {
                Ok(event) => {
                    sender.send(event).unwrap();
                },
                Err(e) => {
                    eprintln!("Failed to discover service: {}", e);
                }
            }
    })?;

    Ok(receiver)
}
// ============================================
//            Get Available Rooms
// ============================================
pub fn get_available_rooms(receiver: Receiver<ServiceEvent>) -> Vec<Room> {
    let mut rooms = Vec::new();
    while let Ok(event) = receiver.recv_timeout(Duration::from_secs(2)) {
        if let ServiceEvent::ServiceResolved(info) = event {
            if let Some(address) = info.get_addresses().iter().next() {
                rooms.push(Room {
                    name: info.get_fullname().to_string(),
                    address: *address,
                    port: info.get_port(),
                    creator_device_id: info.get_property("creator_device_id").unwrap().to_string(),
                    properties: txt_properties_to_hash_map(info.get_properties()),
                });
            }
        }
    }
    rooms
}
// ============================================
//       Convert TxtProperties to HashMap
// ============================================
fn txt_properties_to_hash_map(txt_properties: &TxtProperties) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for txt_property in txt_properties.iter() {
        let key = txt_property.key().to_string();
        let value = txt_property.val_str();
        map.insert(key, value.to_string());
    }
    map
}
// ============================================
//           Select Network Room
// ============================================
pub fn select_network(channels: Vec<Room>) -> Option<Room>{
    channels.into_iter().next()
}
