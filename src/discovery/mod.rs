// ============================================
//                  Imports
// ============================================
// mDNS or custom discovery
#[allow(unused_imports)]
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
#[allow(unused_imports)]
use std::net::{IpAddr, Ipv4Addr};
#[allow(unused_imports)]
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent, Error, TxtProperties, TxtProperty};
use crate::db;
use if_addrs::get_if_addrs;
use dialoguer::{Select, theme::ColorfulTheme};

// ============================================
//                 Structures
// ============================================

#[derive(Debug, Clone)]
pub struct Room {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub creator_device_id: String,
    pub metadata: HashMap<String, serde_json::Value> // This can store user permissions, group memberships, etc.
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
    service_daemon: mdns_sd::ServiceDaemon,
    pool: &db::SqlitePool,
    room_name: &str,
    creator_device_id: &str,
    metadata: HashMap<String, serde_json::Value>,
    port: u16
) -> Result<(ServiceInfo, IpAddr), Box<dyn std::error::Error>> {
    // Serialize metadata to JSON string
    let metadata_str = serde_json::to_string(&metadata)?;

    //Prepare mDNS properties
    let mut txt_properties = HashMap::new();
    txt_properties.insert("metadata".to_string(), metadata_str);

    let mdns = service_daemon;

    // Create ServiceInfo Object
    let service_type_webrtc = "_webrtc._udp.local."; // WebRTC over UDP
    let service_type_websocket = "_ws._tcp.local."; // WebSocket over TCP
    let service_name = format!("{}_by_{}", room_name.to_lowercase(), creator_device_id.to_lowercase());
    let hostname = format!("{}.local.", creator_device_id.to_lowercase());
    let instance_name = room_name.to_string();

    println!("Creating ServiceInfo with the following parameters:");
    println!("Service Type (WebRTC): {}", service_type_webrtc);
    println!("Service Type (WebSocket): {}", service_type_websocket);
    println!("Service Name: {}", service_name);
    println!("Hostname: {}", hostname);
    println!("Instance Name: {}", instance_name);
    println!("Port: {}", port);

    // Allow the user to select a network interface
    let ip_address = match select_network_interface() {
        Some(ip) => ip,
        None => {
            eprintln!("Error: unable to get local IP address");
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unable to get local IP address")
                )
            );
        }
    };

    println!("Selected IP address: {}", ip_address);

    // Convert IP address to a slice
    let ip_addresses = vec![ip_address];

    // Create ServiceInfo Object
    let room_service_info_webrtc = ServiceInfo::new(
        &service_type_webrtc,
        &service_name,
        &hostname,
        ip_addresses.as_slice(),
        port,
        txt_properties.clone(),
    ).map_err(|e| {
            eprintln!("Error creating ServiceInfo (WebRTC): {}", e);
            e
        })?;

    let room_service_info_websocket = ServiceInfo::new(
        &service_type_websocket,
        &service_name,
        &hostname,
        ip_addresses.as_slice(),
        port,
        txt_properties,
    )?;


    // Register service with mDNS Responder
    mdns.register(room_service_info_webrtc.clone())?;
    mdns.register(room_service_info_websocket.clone())?;
    println!("Service registered successfully.");

    // Create Room Object for database storage
    let room = Room {
        name: room_name.to_string(),
        address: ip_address,
        port,
        creator_device_id: creator_device_id.to_string(),
        metadata,
    };
    db::store_room_info(pool, &room);

    Ok((room_service_info_webrtc, ip_address))
}

#[allow(dead_code)]
fn get_local_ip_address() -> Option<IpAddr> {
    let ifaces = if_addrs::get_if_addrs().ok()?;
    for iface in ifaces {
        if iface.is_loopback() {
            continue;
        }
        return Some(iface.addr.ip());
    }
    None
}

fn select_network_interface() -> Option<IpAddr> {
    let ifaces = get_if_addrs().ok()?;
    let non_loopback_ifaces: Vec<_> = ifaces
        .into_iter()
        .filter(|iface| !iface.is_loopback() && matches!(iface.addr, if_addrs::IfAddr::V4(_)))
        .collect();

    let iface_names: Vec<_> = non_loopback_ifaces
        .iter()
        .map(|iface| iface.name.clone())
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a network interface")
        .default(0)
        .items(&iface_names)
        .interact()
        .unwrap();

    let selected_iface = &non_loopback_ifaces[selection];
    match selected_iface.addr.ip() {
        std::net::IpAddr::V4(ipv4) => Some(std::net::IpAddr::V4(ipv4)),
        _ => None,
    }
}


// ============================================
//            Load and Broadcast Services
// ============================================
pub fn load_and_broadcast_services(pool: &db::SqlitePool)
-> Result<(), Box<dyn std::error::Error>> {
    let rooms = db::load_rooms(pool);
    let responder = start_mdns_responder()?;

    for room in rooms {
        let service_name = format!("{}_by_{}", room.name, room.creator_device_id);
        let service_type_webrtc = "_webrtc._udp.local.";
        let service_type_websocket = "_ws._tcp.local.";
        let hostname = format!("{}.local.", room.creator_device_id);
        let metadata = HashMap::new(); // Add actual properties

        // Convert IP addresss to a slice
        let ip_addresses = vec![room.address];

        let service_info_webrtc = ServiceInfo::new(
            &service_type_webrtc,
            &service_name,
            &hostname,
            ip_addresses.as_slice(),
            room.port,
            metadata.clone(),
        )?;

        let service_info_websocket = ServiceInfo::new(
            &service_type_websocket,
            &service_name,
            &hostname,
            ip_addresses.as_slice(),
            room.port,
            metadata.clone(),
        )?;

        responder.register(service_info_webrtc.clone())?;
        responder.register(service_info_websocket.clone())?;
        }
        Ok(())
}
// ============================================
//            Discover Services
// ============================================
pub fn discover_services() -> Result<Receiver<mdns_sd::ServiceEvent>, Box<dyn std::error::Error>> {
    let responder = start_mdns_responder()?;
    let (sender, receiver) = mpsc::channel();

    let service_discovery_webrtc = responder.browse("_webrtc.udp.local.").unwrap();
    let service_discovery_websocket = responder.browse("_ws._tcp.local.").unwrap();

    std::thread::spawn(move || {
        loop {
            match service_discovery_webrtc.recv() {
                Ok(event) => {
                    if sender.send(event).is_err() {
                        break;
                    }
                },
                Err(_) => break,
            }
            match service_discovery_websocket.recv() {
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
                    metadata: txt_metadata_to_hashmap(info.get_properties()),
                });
            }
        }
    }
    rooms
}
// ============================================
//       Convert TxtProperties to HashMap
// ============================================
#[allow(dead_code)]
fn txt_properties_to_hash_map(txt_properties: &TxtProperties) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for txt_property in txt_properties.iter() {
        let key = txt_property.key().to_string();
        let value = txt_property.val_str();
        map.insert(key, value.to_string());
    }
    map
}

fn txt_metadata_to_hashmap(txt_properties: &TxtProperties) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    if let Some(metadata) = txt_properties.get("metadata") {
        if let Ok(parsed_metadata) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&metadata.to_string()) {
            map = parsed_metadata;
        }
    }
    map
}
