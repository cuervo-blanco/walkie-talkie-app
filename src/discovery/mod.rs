// ============================================
//                  Imports
// ============================================
// mDNS or custom discovery
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent, ServiceDaemonError, TxtProperties, TxtProperty};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};

// ============================================
//                 Structures
// ============================================

pub struct Room {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub creator_device_id: String,
    pub properties: HashMap<String, String> // This can store user permissions, group memeberships, etc.
}
// ============================================
//              mDNS Responder
// ============================================
pub fn start_mdns_responder() -> Result<ServiceDaemon, ServiceDaemonError> {
    // Initialize and return an mDNS ServiceDaemon
    ServiceDaemon::new()
}
// ============================================
//            Broadcast Service
// ============================================
pub fn broadcast_service(room_name: &str, creator_device_id: &str, port: u16) -> Result<ServiceInfo, Box<dyn std::error::Error>> {
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

    Ok(service_info)
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
