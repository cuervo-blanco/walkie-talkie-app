use wt_tools::audio;
use wt_tools::communication;
use wt_tools::discovery;
use wt_tools::db;
use wt_tools::metadata;

fn main() {
    // Initializ necessary components
    audio::initialize_audio_interface();
    communication::initialize_webrtc();
    discovery::start_discovery();
    db::initialize_database();
}
