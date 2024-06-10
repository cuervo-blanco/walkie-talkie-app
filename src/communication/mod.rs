// WebRTC logic, data channels
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::data_channel::RTCDataChannel;
use crate::audio::FormattedAudio;

pub struct Destination;

pub fn initialize_webrtc() -> RTCPeerConnection {
    // Initialize WebRTC communication
    todo!()
}

#[allow(unused_variables)]
pub fn create_data_channel(pc: &RTCPeerConnection, label: &str) -> RTCDataChannel {
    // Create a RTCDataChannel
    todo!()
}

#[allow(unused_variables)]
pub fn send_audio_to_destination(audio: FormattedAudio, destination: Destination) {
    // Send audio to the specified destination using WebRTC
}

#[allow(unused_variables)]
pub fn set_destination(dest: Destination) {
    // Set the destination for audio streaming
}
pub fn receive_audio() -> FormattedAudio {
    // Receive audio from WebRTC and return it in a format suitable for a playback
    todo!()
}
