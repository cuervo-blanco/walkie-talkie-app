// ============================================
//          Dependencies and Imports
// ============================================
use std::io;
use std::io::Write;
use wt_tools::audio;
use wt_tools::communication::WebRTCModule;
use wt_tools::discovery;
use wt_tools::db;
use wt_tools::log;
use wt_tools::websocket::WebSocketStream;
use dialoguer::{theme::ColorfulTheme, Select};
use tokio;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    log::log_message("Application Started");
    title_card();

    // Initialize the database connection pool
    let dp_path = "walkie_talkie_app.db";
    let pool = db::initialize_pool(db_path);
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
                let creator_device_id = get_input("Enter your ID: ");
                let port: u16 = get_input("Enter port: ").parse().expect("Invalid port number");

                let metadata = serde_json::json!({
                    "groups":{
                        "all": {
                            "members": {
                                creator_device_id.to_string(): {
                                    "admin": "true",
                                    "online": "true",
                                    "mute": "false"
                                },
                            }
                        },
                    },
                });

                discovery::broadcast_service(&pool, &room_name, &creator_device_id, port, metadata).unwrap();
                start_network_services(&websocket_stream, &webrtc_module, &creator_device_id, port).await;
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
                let items: Vec<String> = Vec::new();
                for room in rooms {
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
                    start_network_services(
                        &websocket_stream,
                        &webrtc_module,
                        &room.creator_device_id,
                        room.port
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
fn title_card() {
    println!("██╗    ██╗ █████╗ ██╗     ██╗  ██╗██╗███████╗    ████████╗ █████╗ ██╗     ██╗  ██╗██╗███████╗");
    println!("██║    ██║██╔══██╗██║     ██║ ██╔╝██║██╔════╝    ╚══██╔══╝██╔══██╗██║     ██║ ██╔╝██║██╔════╝");
    println!("██║ █╗ ██║███████║██║     █████╔╝ ██║█████╗         ██║   ███████║██║     █████╔╝ ██║█████╗");
    println!("██║███╗██║██╔══██║██║     ██╔═██╗ ██║██╔══╝         ██║   ██╔══██║██║     ██╔═██╗ ██║██╔══╝");
    println!("╚███╔███╔╝██║  ██║███████╗██║  ██╗██║███████╗       ██║   ██║  ██║███████╗██║  ██╗██║███████╗");
    println!(" ╚══╝╚══╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝╚══════╝       ╚═╝   ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝╚══════╝");

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
    websocket_stream: &WebSocketStream,
    webrtc_module: &WebRTCModule,
    device_id: &str,
    port: u16,
) {
    let server_addr = format!("{}:{}", device_id, port);
    tokio::spawn(async move {
        websocket_stream.start(&server_addr).await;
    });

    let signaling_url = format!("ws://{}:{}", device_id, port);
    tokio::spawn(async move {
        // Give the initial groups
        webrtc_module.signaling_loop(&signaling_url, "peer_id").await
        .expect("Signaling loop failed");
    });

    let (input_device, output_device) = audio::initialize_audio_interface();
    if let (Some(input_device), Some(output_device)) = (input_device, output_device) {
        let input_config = audio::get_audio_config(&input_device).expect("Failed to get audio input config");
        let output_config = audio::get_audio_config(&output_device).expect("Failed to get audio output config");

        let audio_buffer = Arc::new(Mutex::new(Vec::new()));
        let received_data = Arc::clone(&audio_buffer);

        let input_stream = audio::start_input_stream(&input_device, &input_config)
            .expect("Failed to start input stream");
        let output_stream = audio::start_output_stream(
                &output_device,
                &output_config,
                received_data.clone()
            ).expect("Failed to start output stream");

        // Simulate sending audio data
        let send_audio_module =  webrtc_module.clone();
        tokio::spawn(async move {
            loop {
                // Encode and send audio data
                let buffer = audio_buffer.lock().unwrap();
                if !buffer.is_empty() {
                    // Replace withactual WebRTC send function
                    let opus_data = audio::convert_audio_stream_to_opus(&buffer)
                        .expect("Failed to encode audio");
                    send_audio_module.send_audio(Ok(opus_data), "group").await
                        .expect("Failed to send audio");
                }
            }
        });

        // Stop the audio streams on exit
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        audio::stop_audio_stream(input_stream);
        audio::stop_audio_stream(output_stream);
    } else {
        log::log_message("Failed to initialize audio devices.");
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
