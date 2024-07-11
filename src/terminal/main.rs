// ============================================
//          Dependencies and Imports
// ============================================
use std::io;
use std::io::Write;
use rand::Rng;
#[allow(unused_imports)]
use wt_tools::audio;
use wt_tools::communication::WebRTCModule;
use wt_tools::communication;
use wt_tools::discovery;
use wt_tools::db;
use wt_tools::log;
use wt_tools::metadata;
use wt_tools::websocket;
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
    let mdns = discovery::start_mdns_responder().unwrap();

    // considerar usar HashMap
    let running_rooms = Arc::new(Mutex::new(Vec::new()));

    loop {
        // Display the main menu
        let selection = main_menu().await;
        match selection {
            0 => {
                // ============================================
                //          Create Room
                // ============================================
                let room_name = get_input("Enter room name: ");
                let creator_device_id = get_input("Enter your username: ");

                let metadata = serde_json::json!({
                    // List of all groups
                    "groups":{
                        // Groups names
                        // The all group will contain everyone by default
                        "all": {
                            // Members of the group
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
                
                // Generate random port number
                fn generate_port_number() -> u16 {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(0..=65535)
                }
                let port = generate_port_number();

                //----TODO----//
                // Check how we can know if the port is available

                // Save info to database
                //Convert the metadata into a hashmap
                let metadata_map = metadata::json_to_metadata(&metadata.to_string());
                let (_, ip_address) = discovery::broadcast_service(
                    mdns.clone(),
                    &pool,
                    &room_name,
                    &creator_device_id,
                    metadata_map.clone(),
                    port
                ).unwrap();

                let websocket_stream_clone = websocket_stream.clone();
                let mut webrtc_module_clone = webrtc_module.clone();
                let creator_device_id_clone = creator_device_id.clone();

                let addr = format!("{}:{}", ip_address, port); // Use the selected network interface here

                let room_task = tokio::spawn(async move {
                    websocket_stream_clone.start(&addr).await;
                    let ws_addr = format!("ws://{}", addr);
                    webrtc_module_clone.signaling_loop(
                        &ws_addr,
                        &creator_device_id_clone,
                        vec!["all".to_string()]
                    ).await.unwrap();
                });

                running_rooms.lock().unwrap().push(room_task);

                room_menu().await;

            }
            1 => {
                // ============================================
                //          Discover Rooms
                // ============================================
                let selected_ip = discovery::select_network_interface().unwrap();
                let receiver = discovery::discover_services(selected_ip).unwrap();
                let rooms = discovery::get_available_rooms(receiver);
                display_rooms_to_user(&rooms);
            }
            2 => {
                // ============================================
                //          Join Room
                // ============================================
                join_room(&websocket_stream, &webrtc_module, &pool).await;
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
    repeat_println("", 3, 100).await;
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
    repeat_println("", 15, 100).await;
}
// ============================================
//          Main Menu Function
// ============================================

async fn repeat_println(line: &str, times: u16, duration: u64) {
    for _ in 0..times {
    sleep(Duration::from_millis(duration)).await;
    println!("{}", line);
    }
}

async fn main_menu() -> usize {
    let selections = &[
        "Create Room",
        "Discover Rooms",
        "Join Rooms",
        "Exit"
    ];

    // Display each item with a delay
    // display_with_delay(selections, 100).await;

    print!("{}[2J", 27 as char);

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select an option")
        .default(0)
        .items(&selections[..])
        .interact()
        .unwrap();

    return selection

}
#[allow(dead_code)]
async fn display_with_delay(items: &[&str], delay_ms: u64) {
    for item in items {
        println!("{}", item);
        sleep(Duration::from_millis(delay_ms)).await;
    }
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
    websocket_stream: &WebSocketStream,
    webrtc_module: &WebRTCModule,
    ws_url: &str,
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

    let device_id = get_input("Enter your username: ");

    let mut webrtc_module = webrtc_module.clone();
    let addr = ws_url.replace("ws://","");
    websocket_stream.start(&addr).await;
    webrtc_module.signaling_loop(
        ws_url,
        &device_id, 
        initial_groups).await.unwrap();
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

// ============================================
//          Room Menu Function
// ============================================
async fn room_menu() {
    loop {
        let selections = &[
            "Select Group",
            "Create Group",
            "Back to Main Menu",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Room Menu")
            .default(0)
            .items(&selections[..])
            .interact()
            .unwrap();

        match selection {
            0 => {
                // TODO: Display available groups
                // TODO: Send Audio to Group
                // TODO: Implement send audio to group
                todo!()
            }
            1 => {
                // Create Group
                // TODO: Implement creating group (check if user has creation permits)
                todo!()
            }
            2 => {
                break;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }
}

async fn join_room(
    websocket_stream: &websocket::WebSocketStream,
    webrtc_module: &communication::WebRTCModule,
    pool: &db::SqlitePool
) {
    let ws_url = get_input("Enter the WebSocket URL (ws://...): ");
    let metadata = db::get_room_metadata(pool, &ws_url).unwrap_or_default();

    start_network_services(
        websocket_stream,
        webrtc_module,
        &ws_url,
        metadata,
    ).await;

}

