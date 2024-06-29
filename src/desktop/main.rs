use wt_tools::audio;
use wt_tools::communication;
use wt_tools::discovery;
use wt_tools::db;
#[allow(unused_imports)]
use wt_tools::metadata;
use wt_tools::log;
use wt_tools::websocket;
use tokio;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize the logger
    log::log_message("Application Started");

    // Initialize the database connection pool
    let db_path = "walkie_talkie_app.db";
    let pool = initialize_pool(db_path);
    initialize_database(&pool);

    // Placeholder for front-end integration
    // Here we could handle user input from the GUI and interact with the backend
    // Example:
    // let user_action = get_user_action_grom_gui();
    // handle_user_action(user_action, &websocket_stream, &webrtc_module, &pool).await;

    // Placeholder for discovering and broadcasting services
    // example:
    // let discover = true;
    // if discover {
    //      let rooms = discovery::get_available_rooms(discovery::discover_services().unwrap());
    //      display_rooms_to_user(rooms);
    //} else {
    //      let room_name = "Example Room";
    //      let creator_device_id = "Device1234";
    //      let port = 8080;
    //      discovery::broadcast_service(
    //          &pool,
    //          room_name,
    //          creator_device_id,
    //          port,
    //      ).unwrap();
    //}

    // Initialize WebSocket and WebRTC modules
    let websocket_stream = websocket::WebSocketStream::new(pool.clone());
    let webrtc_module = WebRTCModule::new(&pool).await.unwrap();

    // Keep the main application running
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");

    log::log_message("Application stopped");

    Ok(())
}
// ============================================
//       Placeholder Functions for GUI
// ============================================

async fn handle_user_action(
    action: UserAction,
    websocket_stream: &WebSocketStream,
    webrtc_module: &WebRTCModule,
    pool: &SqlitePool) {

    match action {
        UserAction::CreateRoom { room_name, creator_device_id, port} => {
            discovery::broadcast_service(
                pool,
                &room_name,
                &creator_device_id,
                port,
            ).unwrap();
            start_network_services(
                websocket_stream,
                webrtc_module,
                &creator_device_id,
                port,
            ).await;
        }
        UserAction::DiscoverRooms => {
            let receiver = discovery::discover_services().unwrap();
            let rooms = discovery::get_available_rooms(receiver);
            display_rooms_to_user(rooms);
        }
        UserAction::JoinRoom { room } => {
            start_network_services(
                websocket_stream,
                webrtc_module,
                &room.creator_device_id,
                room.port,
            ).await;
        }
        UserAction::MuteUser { user_id, room } => {
            // Logic to mute a user
        }
        UserAction::CreateGroup { group_name, room } => {
            // Logic to create a group
        }
        // Add other actions as needed
    }
}
enum UserAction {
    CreateRoom {room_name: String, creator_device_id: String, port: u16 },
    DiscoverRooms,
    JoinRoom { room: Room },
    MuteUser { user_id: String, room: Room },
    CreateGroup { group_name: String, room: Room },
}

// ============================================
//       Placeholder Functions for GUI
// ============================================

fn display_rooms_to_user(rooms: Vec<Room>) {
    // Implement the logic to display available rooms to the user
}

fn get_user_action_from_gui() -> UserAction {
    // Implement the logic to get user action from the GUI
}
// ============================================
//       Start Network Services for a Room
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
        webrtc_module.signaling_loop(&signaling_url, "peer_id").await
        .expect("Signaling loop failed");
    });

    let (input_device, output_device) = initialize_audio_interface();
    if let (Some(input_device), Some(output_device)) = (input_device, output_device) {
        let input_config = get_audio_config(&input_device).expect("Failed to get audio input config");
        let output_config = get_audio_config(&output_device).expect("Failed to get audio output config");

        let audio_buffer = Arc::new(Mutex::new(Vec::new()));
        let received_data = Arc::clone(&audio_buffer);

        let input_stream = start_input_stream(&input_device, &input_config)
            .expect("Failed to start input stream");
        let output_stream = start_output_stream(
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
