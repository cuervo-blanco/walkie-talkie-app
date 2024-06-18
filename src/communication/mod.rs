// WebRTC logic, data channels
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::policy::ice_transport_policy::RTCIceTransportPolicy;
#[allow(unused_imports)]
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::peer_connection::policy::bundle_policy::RTCBundlePolicy;
use webrtc::peer_connection::policy::rtcp_mux_policy::RTCRtcpMuxPolicy;
#[allow(unused_imports)]
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
#[allow(unused_imports)]
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::Error;
use futures::{SinkExt, StreamExt};
use futures::channel::mpsc;
use crate::audio::FormattedAudio;
use crate::log;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::sync::{Arc, Mutex};
use bytes::Bytes;
use std::collections::HashMap;

pub struct Destination;

pub struct WebRTCModule {
    api: webrtc::api::API,
    peer_connections: HashMap<String, RTCPeerConnection>,
    audio_data_channels: Vec<Arc<RTCDataChannel>>,
}

impl WebRTCModule {
    pub async fn new() -> Result<Self, webrtc::Error> {
        // Initialize WebRTC communication
        let api = create_api().await?;
        // Additional WebRTC logic here (e.g creating offers/answers, adding tracks, etc.)
        Ok(Self{
            api,
            peer_connections: HashMap::new(),
            audio_data_channels: Vec::new(),
        })
    }
    pub async fn signaling_loop(&mut self, signaling_url: &str, peer_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // The connect_async simply connects to a given url which in this case is of ws:// type
        let (mut ws_stream, _) = connect_async(signaling_url).await.expect("Failed to connect");
        // Register with the signaling server
        ws_stream.send(Message::Text(format!("register:{}", peer_id))).await?;
        // Receive initial list of peers in the channel
        if let Some(Ok(Message::Text(peer_list))) = ws_stream.next().await {
            let peers: Vec<String> = peer_list.split(',').map(|s| s.to_string()).collect();
            // Create an offer for each peer and send it
            for peer in peers {
                let peer_connection = create_peer_connection(&self.api, &mut self.audio_data_channels).await?;
                let offer_sdp = create_offer(&peer_connection).await?;
                self.peer_connections.insert(peer.clone(), peer_connection);
                ws_stream.send(Message::Text(format!("{}:offer:{}:{}", peer_id, peer, offer_sdp))).await?;
            }
        }
        // Recieve messages from the signaling server
        while let Some(message) = ws_stream.next().await{
            match message? {
                Message::Text(text) => {
                    if text.starts_with("new_peer:") {
                        let new_peer_id = &text[9..];
                        //Create an offer for the new peer
                        let peer_connection = create_peer_connection(&self.api, &mut self.audio_data_channels).await?;
                        let offer_sdp = create_offer(&peer_connection).await?;
                        self.peer_connections.insert(new_peer_id.to_string(), peer_connection);
                        ws_stream.send(Message::Text(format!("{}:offer:{}:{}", peer_id, new_peer_id, offer_sdp))).await?;
                    } else if text.starts_with("offer:") {
                        let parts: Vec<&str> = text.splitn(3, ':').collect();
                        let message_type = parts[0];
                        let remote_peer_id = parts[1];
                        let sdp_or_candidate = parts[2];

                        match message_type {
                            "offer" => {
                                let peer_connection = create_peer_connection(&self.api, &mut self.audio_data_channels).await?;
                                set_remote_description(&peer_connection, RTCSessionDescription::offer(sdp_or_candidate.to_string())?).await?;
                                let answer_sdp = create_answer(&peer_connection).await?;
                                self.peer_connections.insert(remote_peer_id.to_string(), peer_connection);
                                ws_stream.send(Message::Text(format!("{}:answer:{}:{}", remote_peer_id, peer_id, answer_sdp))).await?;
                            }
                            "answer" => {
                                if let Some(peer_connection) = self.peer_connections.get_mut(remote_peer_id) {
                                    set_remote_description(&peer_connection, RTCSessionDescription::answer(sdp_or_candidate.to_string())?).await?;
                                }
                            }
                            "candidate" => {
                                let ice_candidate = RTCIceCandidateInit {
                                    candidate: sdp_or_candidate.to_string(),
                                    sdp_mid: None,
                                    sdp_mline_index: None,
                                    username_fragment: None,
                                };
                                // Add ICE Candidate to the peer_connection
                                if let Some(peer_connection) = self.peer_connections.get_mut(remote_peer_id) {
                                    add_ice_candidate(peer_connection, ice_candidate).await?;
                                }
                            }
                            _ => {}
                        }
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }
    pub async fn send_audio(&self, audio_data: FormattedAudio) -> Result<(), Box<dyn std::error::Error>>{
        // Check if the audio data is valid
        if let Ok(data) = audio_data {
            let bytes = Bytes::from(data);
            // Send audio to the specified destination using WebRTC
            for data_channel in &self.audio_data_channels {
                if let Ok(_) = data_channel.send(&bytes).await {
                    log::log_message("Audio data sent successfully");
                } else {
                    log::log_message("Failed to send audio data");
                }
            }
        } else {
            log::log_message("Invalid audio data");
        }
        Ok(())
    }
    pub async fn receive_audio(&self) -> mpsc::Receiver<Vec<u8>> {
        let (sender, receiver) = mpsc::channel(100); // Channel to send audio data

        for data_channel in &self.audio_data_channels {
            let mut sender_clone = sender.clone();
            let data_channel_clone = Arc::clone(data_channel);
            data_channel_clone.on_message(Box::new(move |msg: DataChannelMessage| {
                let data = msg.data.to_vec();
                // Send the data through the channel
                if let Err(_) = sender_clone.try_send(data) {
                    log::log_message("Failed to send received audio data");
                }
                Box::pin(async {})
            }));
        }
        receiver
    }
}

async fn create_peer_connection(api: &webrtc::api::API, audio_data_channels: &mut Vec<Arc<RTCDataChannel>>) -> Result<RTCPeerConnection, Error> {
    let config = create_rtc_configuration();
    let peer_connection = api.new_peer_connection(config).await?;
    // Create a data channel for audio
    let data_channel_init = RTCDataChannelInit {
        ordered: Some(true),
        ..Default::default()
    };
    let audio_data_channel = peer_connection.create_data_channel("audio", Some(data_channel_init)).await?;
    audio_data_channels.push(audio_data_channel);
    Ok(peer_connection)
}
async fn create_media_engine() -> Result<MediaEngine, webrtc::Error>  {
    let mut media_engine = MediaEngine::default();
    // Register default codecs including Opus
    match media_engine.register_default_codecs() {
        Ok(_) => {
            log::log_message(&format!("Successfuly registred default codecs"));
            Ok(media_engine)
        },
        Err(e) => {
            log::log_message(&format!("Error encountered registring default codecs: {}", e));
            Err(e)
        }
    }
}
async fn create_api() -> Result<webrtc::api::API, Error> {
    let media_engine = match create_media_engine().await {
        Ok(engine) => {
            log::log_message(&format!("Successful creation of Media Engine"));
            engine
        },
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
async fn create_offer(peer_connection: &RTCPeerConnection) -> Result<String, Error> {
    // First step when communicating with peer
    let offer = peer_connection.create_offer(None).await?;

    peer_connection.set_local_description(offer.clone()).await?;

    Ok(offer.sdp)
}
async fn create_answer(peer_connection: &RTCPeerConnection) -> Result<String, Error> {
    // Response to peer upon receiving offer (sends answer)
    let answer = peer_connection.create_answer(None).await?;

    peer_connection.set_local_description(answer.clone()).await?;

    Ok(answer.sdp)
}
async fn set_remote_description(peer_connection: &RTCPeerConnection, received_offer: RTCSessionDescription) -> Result<(), Error> {
    // Receives offer and sets it as the remote description
    peer_connection.set_remote_description(received_offer).await?;
    Ok(())
}
async fn add_ice_candidate(peer_connection: &RTCPeerConnection, candidate: RTCIceCandidateInit) -> Result<(), Error> {
    // Adds Received ICE candidate
    peer_connection.add_ice_candidate(candidate).await?;
    Ok(())
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
