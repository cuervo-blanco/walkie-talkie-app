// ============================================
//          Dependencies and Imports
// ============================================
use std::io;
use std::io::Write;
#[allow(unused_imports)]
use wt_tools::audio;
use wt_tools::communication::WebRTCModule;
use wt_tools::discovery;
use wt_tools::db;
use wt_tools::log;
use wt_tools::metadata;
use wt_tools::websocket::WebSocketStream;
use dialoguer::{theme::ColorfulTheme, Select};
use tokio;
use tokio::time::{sleep, Duration};
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    log::log_message("Application Started");
    title_card().await;

    // Initialize the database connection pool
    let dp_path = "walkie_talkie_app.db";
    let pool = db::initialize_pool(dp_path);
    db::initialize_database(&pool);

    // Initialize WebSocket and WebRTC modules
    let websocket_stream = WebSocketStream::new(pool.clone());
    let webrtc_module = WebRTCModule::new(&pool).await.unwrap();

    loop {
        // Display the menu
        let selection = main_menu();
        match selection {
            0 => {
                // ============================================
                //          Create Room
                // ============================================
                let room_name = get_input("Enter room name: ");
                let creator_device_id = get_input("Enter creator device ID: ");
                let port: u16 = get_input("Enter port: ").parse().expect("Invalid port number");

                let metadata = serde_json::json!({
                    "groups":{
                        "all": {
                            "members": {
                                creator_device_id.clone(): {
                                    "admin": true,
                                    "online": true,
                                    "mute": false
                                },
                            }
                        },
                    },
                });
                // Save info to database

                let metadata_map = metadata::json_to_metadata(&metadata.to_string());
                discovery::broadcast_service(&pool, &room_name, &creator_device_id, port, metadata_map.clone()).unwrap();

                start_network_services(&websocket_stream, &webrtc_module, &creator_device_id, port, metadata_map).await;

                //========= start audio input and handling process ==========//
                //
                //              TODO
                //              1. Find how to keep this process alive if the
                //              decides to create more rooms (go to the previous
                //              menu
                //              2. Start audio broadcasting menu (send audio,
                //              select group to send audio, etc.)
                //
                //
                //
                //===========================================================//
            }
            1 => {
                // ============================================
                //          Discover Rooms
                // ============================================
                let receiver = discovery::discover_services().unwrap();
                let rooms = discovery::get_available_rooms(receiver);
                display_rooms_to_user(&rooms);
            }
            2 => {
                // ============================================
                //          Join Room
                // ============================================
                let receiver = discovery::discover_services().unwrap();
                let rooms = discovery::get_available_rooms(receiver);
                let mut items: Vec<String> = Vec::new();
                for room in rooms.clone() {
                    items.push(room.name);
                }

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select a Room: ")
                    .default(0)
                    .items(&items[..])
                    .interact()
                    .unwrap();
                if selection > 0 && selection <= rooms.len() {
                    let room = &rooms[selection - 1];
                    let metadata = serde_json::json!(room.metadata).clone();
                    let metadata_map = metadata::json_to_metadata(&metadata.to_string());
                    start_network_services(
                        &websocket_stream,
                        &webrtc_module,
                        &room.creator_device_id,
                        room.port,
                        metadata_map,
                    ).await;
                } else {
                    println!("Invalid room");
                }
            }
            3 => {
                // ============================================
                //          Exit Application
                // ============================================
                break;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
    log::log_message("Application stopped");
    Ok(())
}

// ============================================
//          Title Card Function
// ============================================
async fn title_card() {
    println!("");
    sleep(Duration::from_millis(100)).await;
    println!("");
    sleep(Duration::from_millis(100)).await;
    println!("");
    sleep(Duration::from_millis(100)).await;
    println!("██╗    ██╗ █████╗ ██╗     ██╗  ██╗██╗███████╗    ████████╗ █████╗ ██╗     ██╗  ██╗██╗███████╗");
    sleep(Duration::from_millis(100)).await;
    println!("██║    ██║██╔══██╗██║     ██║ ██╔╝██║██╔════╝    ╚══██╔══╝██╔══██╗██║     ██║ ██╔╝██║██╔════╝");
    sleep(Duration::from_millis(100)).await;
    println!("██║ █╗ ██║███████║██║     █████╔╝ ██║█████╗         ██║   ███████║██║     █████╔╝ ██║█████╗");
    sleep(Duration::from_millis(100)).await;
    println!("██║███╗██║██╔══██║██║     ██╔═██╗ ██║██╔══╝         ██║   ██╔══██║██║     ██╔═██╗ ██║██╔══╝");
    sleep(Duration::from_millis(100)).await;
    println!("╚███╔███╔╝██║  ██║███████╗██║  ██╗██║███████╗       ██║   ██║  ██║███████╗██║  ██╗██║███████╗");
    sleep(Duration::from_millis(100)).await;
    println!(" ╚══╝╚══╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝╚══════╝       ╚═╝   ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝╚══════╝");
    sleep(Duration::from_millis(100)).await;
    println!("");
    sleep(Duration::from_millis(100)).await;
    println!("");
    sleep(Duration::from_millis(100)).await;
    println!("");

}
// ============================================
//          Main Menu Function
// ============================================
fn main_menu() -> usize {
    let selections = &[
        "Create Room",
        "Discover Rooms",
        "Join Rooms",
        "Exit"
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select an option")
        .default(0)
        .items(&selections[..])
        .interact()
        .unwrap();

    return selection

}
// ============================================
//          Get User Input Function
// ============================================
fn get_input(prompt: &str) -> String {
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read input");
    input.trim().to_string()
}
// ============================================
//          Start Network Services Function
// ============================================
async fn start_network_services(
    _websocket_stream: &WebSocketStream,
    _webrtc_module: &WebRTCModule,
    _device_id: &str,
    _port: u16,
    metadata: std::collections::HashMap<String, serde_json::Value>,
) {
    let mut initial_groups: Vec<String> = Vec::new();
    if let Some(groups) = metadata::find_metadata_value(&metadata, "groups") {
        if let Some(groups_map) = groups.as_object() {
            for (group_name, _) in groups_map {
                initial_groups.push(group_name.clone());
            }
        }
    }
}
// ============================================
//          Display Rooms Function
// ============================================
fn display_rooms_to_user(rooms: &[discovery::Room]) {
    if rooms.is_empty() {
        println!("No rooms available.");
        return;
    }

    println!("Available rooms:");
    for (index, room) in rooms.iter().enumerate() {
        println!("{}: {} at {}:{}", index + 1, room.name, room.address, room.port);
    }
}
