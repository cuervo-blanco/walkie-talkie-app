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
    // Discover available networks using mDNS
    let networks = discovery::discover_networks();
    // User selects a network
    if let Some(selected_network) = discovery::select_network(&networks) {
        log::log_message(&format!("Selected network: {}", selected_network));

        // Initialize WebSocket server on the selected network
        let websocket_server = websocket::WebSocketStream::new();
        let server_addr = format!("{}:8080", selected_network);
        tokio::spawn(async move {
            websocket_server.start(&server_addr).await;
        });

        // Initialize WebRTC module
        let webrtc_module = communication::WebRTCModule::new().await.expect("Failed to initialize WebRTC module");

        // Start the signaling loop for a peer
        let signaling_url = format!("ws://{}:8080", selected_network);
        tokio::spawn(async move {
            webrtc_module.signaling_loop(&signaling_url, "peer_id").await.expect("Signaling loop failed");
        });

        // Initialize audio interface
        let (input_device, output_device) = audio::initialize_audio_interface();

        if let (Some(input_device), Some(output_device)) = (input_device, output_device) {
            // Get audio configurations
            let input_config = audio::get_audio_config(&input_device).expect("Failed to get input config");
            let output_config = audio::get_audio_config(&output_device).expect("Failed to get output config");

            // Create shared buffers for audio data
            let audio_buffer = Arc::new(Mutex::new(Vec::new()));
            let received_data = Arc::clone(&audio_buffer);

            // Start input and output streams
            let input_stream = audio::start_input_stream(&input_device, &input_config).expect("Failed to start input stream");
            let output_stream = audio::start_output_stream(&output_device, &output_config, received_data).expect("Failed to start output stream");

            // Simulate sending audio data
            tokio::spawn(async move {
                loop {
                    // Encode and send audio data
                    let buffer = audio_buffer.lock().unwrap();
                    if !buffer.is_empty() {
                        // Repalce with actual WebRTC send function
                        let opus_data = audio::convert_audio_stream_to_opus(&buffer).expect("Failed to encode audio");
                        webrtc_module.send_audio(opus_data).await.expect("Failed to send audio");
                    }
                }
            });

            // Keep the main application running
            tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
            audio::stop_audio_stream(input_stream);
            audio::stop_audio_stream(output_stream);
        } else {
            log::log_message("Failed to initialize audio devices.");
        }
    } else {
        log::log_message("No network selected or no available networks found.");
    }

    // Initialize the WebRTC connection

    // Create data channels

    // Start mDNS responder

    // Initialize the SQLite database

    log::log_message("Starting the program");

    log::log_message("Finished the program");
    Ok(())
}

