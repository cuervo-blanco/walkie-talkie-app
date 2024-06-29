// ============================================
//                  Imports
// ============================================
// mDNS or custom discovery
#[allow(unused_imports)]
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent, Error, TxtProperties, TxtProperty};
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
pub fn start_mdns_responder() -> Result<ServiceDaemon, Error> {
    // Initialize and return an mDNS ServiceDaemon
    ServiceDaemon::new()
}
// ============================================
//            Broadcast Service
// ============================================
pub fn broadcast_service(
    pool: &db::SqlitePool,
    room_name: &str,
    creator_device_id: &str,
    port: u16,
) -> Result<ServiceInfo, Box<dyn std::error::Error>> {
    // Logic to broadcast mDNS service
    let service_name = format!("{}_{}", room_name, creator_device_id);
    let service_type = "_http._tcp.local"; // Service type for mDNS
    let hostname = format!("{}.local.", creator_device_id);
    let instance_name = room_name.to_string();
    let properties = HashMap::new(); // Permissions, etc.


    // Create ServiceInfo Object
    let service_info = ServiceInfo::new(
        &service_type,
        &service_name,
        &hostname,
        &instance_name,
        port,
        properties.clone(),
    )?;

    // Register service with mDNS Responder
    let responder = start_mdns_responder()?;
    responder.register(service_info.clone())?;

    // Create Room Object for database storage
    let room = Room {
        name: room_name.to_string(),
        address: get_local_ip_address().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)), //Replace with actual address retrieval
        port,
        creator_device_id: creator_device_id.to_string(),
        properties, // Add actual properties
    };
    db::store_room_info(pool, &room);

    Ok(service_info)
}

fn get_local_ip_address() -> Option<IpAddr> {
    let ifaces = get_if_addrs().unwrap();
    for iface in ifaces {
        if iface.is_loopback() {
            continue;
        }
        return Some(iface.addr.ip());
    }
    None
}

// ============================================
//            Load and Broadcast Services
// ============================================
pub fn load_and_broadcast_services(pool: &db::SqlitePool)
-> Result<(), Box<dyn std::error::Error>> {
    let rooms = db::load_rooms(pool);
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
//            Discover Services
// ============================================
pub fn discover_services() -> Result<Receiver<mdns_sd::ServiceEvent>, Box<dyn std::error::Error>> {
    let responder = start_mdns_responder()?;
    let (sender, receiver) = mpsc::channel();

    let service_discovery = responder.browse("_http._tcp.local").unwrap();

    std::thread::spawn(move || {
        loop {
            match service_discovery.recv() {
                Ok(event) => {
                    if sender.send(event).is_err() {
                        break;
                    }
                },
                Err(_) => break,
            }
        }
    });
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
pub fn select_service(_channels: Vec<Room>) -> Option<Room>{
    todo!()
}
