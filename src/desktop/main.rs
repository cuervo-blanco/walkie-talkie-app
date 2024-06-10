use wt_tools::audio;
use wt_tools::communication;
use wt_tools::discovery;
use wt_tools::db;
#[allow(unused_imports)]
use wt_tools::metadata;
use wt_tools::log;

fn main() -> std::io::Result<()> {
    // Initialize necessary components //

    // Initialize Audio Interface
    audio::initialize_audio_interface();

    // Initialize the WebRTC connection
    let pc = communication::initialize_webrtc();

    // Create data channels
    let _all_channel = communication::create_data_channel(&pc, "all");

    // Start mDNS responder
    discovery::start_mdns_responder();

    // Initialize the SQLite database
    let _conn = db::initialize_database();

    log::log_message("Starting the program");

    log::log_message("Finished the program");
    Ok(())
}
