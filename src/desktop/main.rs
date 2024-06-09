use wt_tools::audio;
use wt_tools::communication;
use wt_tools::discovery;
use wt_tools::db;
#[allow(unused_imports)]
use wt_tools::metadata;
use wt_tools::log;

fn main() -> std::io::Result<()> {
    // Initialize necessary components
    audio::initialize_audio_interface();
    communication::initialize_webrtc();
    discovery::start_discovery();
    db::initialize_database();

    log::log_message("Starting the program");

    log::log_message("Finished the program");
    Ok(())
}
