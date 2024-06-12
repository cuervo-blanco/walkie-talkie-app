// WebRTC logic, data channels
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::policy::ice_transport_policy::RTCIceTransportPolicy;
use webrtc::peer_connection::policy::bundle_policy::RTCBundlePolicy;
use webrtc::peer_connection::policy::rtcp_mux_policy::RTCRtcpMuxPolicy;
#[allow(unused_imports)]
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
#[allow(unused_imports)]
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::data_channel::RTCDataChannel;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::Error;
use crate::audio::FormattedAudio;
use crate::log;

pub struct Destination;

async fn create_media_engine() -> Result<MediaEngine, webrtc::Error>  {
    let mut media_engine = MediaEngine::default();
    // Register default codecs including Opus
    match media_engine.register_default_codecs() {
        Ok(_) => Ok(media_engine),
        Err(e) => {
            log::log_message(&format!("Error encountered registring default codecs: {}", e));
            Err(e)
        }
    }
}

async fn create_api() -> Result<webrtc::api::API, Error> {
    let media_engine = match create_media_engine().await {
        Ok(engine) => engine,
        Err(e) => {
            log::log_message(&format!("Error creating media engine: {}", e));
            return Err(e)
        }
    };

    let api = APIBuilder::new().with_media_engine(media_engine).build();
    Ok(api)
}

fn create_rtc_configuration() -> RTCConfiguration {
    RTCConfiguration {
        // Maybe add STUN server in future
        ice_servers: vec![],
        ice_transport_policy: RTCIceTransportPolicy::All,
        bundle_policy: RTCBundlePolicy::MaxBundle,
        rtcp_mux_policy: RTCRtcpMuxPolicy::Require,
        ice_candidate_pool_size: 0,
        // peer_identity & certificates ommited
        ..Default::default()
    }
}

pub async fn create_peer_connection(api: &webrtc::api::API) -> Result<RTCPeerConnection, Error> {
    let config = create_rtc_configuration();
    let peer_connection = api.new_peer_connection(config).await?;
    Ok(peer_connection)
}

#[allow(dead_code)]
async fn handle_peer_connection_events(peer_connection: &RTCPeerConnection) {
    #[allow(unused_variables)]
    let on_connection_state_change = peer_connection.on_peer_connection_state_change(Box::new(
        | state: RTCPeerConnectionState| {
            println!("Peer Connection State had changed: {}", state);
            Box::pin(async {})
        },
    ));
    // Additional event handlers can be added here
}

pub async fn initialize_webrtc() -> Result<webrtc::api::API, Error> {
    // Initialize WebRTC communication
    let api = create_api().await?;
    // Additional WebRTC logic here (e.g creating offers/answers, adding tracks, etc.)
    Ok(api)
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
