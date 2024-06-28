// ============================================
//                  Imports
// ============================================
use bytes::Bytes;
use tokio_tungstenite::MaybeTlsStream;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use crate::audio::FormattedAudio;
use crate::log;
use futures::{SinkExt, StreamExt};
use futures::stream::SplitSink;
use futures::channel::mpsc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::Error;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::policy::bundle_policy::RTCBundlePolicy;
use webrtc::peer_connection::policy::ice_transport_policy::RTCIceTransportPolicy;
use webrtc::peer_connection::policy::rtcp_mux_policy::RTCRtcpMuxPolicy;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

// ============================================
//                 Structures
// ============================================

pub struct Destination;

pub struct WebRTCModule {
    api: webrtc::api::API,
    // Peer Connections: <Name, PeerConnection>
    peer_connections: HashMap<String, RTCPeerConnection>,
    // Audio Data Channels: <Group, DataChannel>
    audio_data_channels: HashMap<String, Vec<Arc<RTCDataChannel>>>,
    audio_sending_active: Arc<Mutex<bool>>,
    audio_receiving_active: Arc<Mutex<bool>>,
    // Peer Groups: <PeerId, Group Membership>
    peer_groups: HashMap<String, Vec<String>>,
    ws_sink: Option<Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::Message>>>>
}

// ============================================
//              Implementation
// ============================================

impl WebRTCModule {
    pub async fn new() -> Result<Self, webrtc::Error> {
        // Initialize WebRTC communication
        let api = create_api().await?;
        // Additional WebRTC logic here (e.g creating offers/answers, adding tracks, etc.)
        Ok(Self{
            api,
            peer_connections: HashMap::new(),
            audio_data_channels: HashMap::new(),
            audio_sending_active: Arc::new(Mutex::new(true)),
            audio_receiving_active: Arc::new(Mutex::new(true)),
            peer_groups: HashMap::new(),
            ws_sink: None
        })
    }
    // Group management
    pub async fn join_group(&mut self, group: &str, data_channel: Arc<RTCDataChannel>) {
        self.audio_data_channels.entry(group.to_string())
            .or_insert_with(Vec::new)
            .push(data_channel);
    }
    pub async fn leave_group(&mut self, group: &str, data_channel: Arc<RTCDataChannel>) {
        if let Some(data_channels) = self.audio_data_channels.get_mut(group) {
            data_channels.retain(|dc| !Arc::ptr_eq(dc, &data_channel));
        }
    }
    pub async fn update_user_groups(&mut self, peer_id: &str, new_groups: Vec<String>
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.peer_groups.insert(peer_id.to_string(), new_groups.clone());

        let group_update_message = format!("group_update:{}:{}",
            peer_id, new_groups.join(",")
        );
        self.broadcast_message(&group_update_message).await?;
        Ok(())
    }
    // Broadcasting
    async fn broadcast_message(&self, message: &str)
    -> Result<(), Box<dyn std::error::Error>> {
        let ws_sink_clone = self.ws_sink.clone().expect("Unable to clone WebRTC Stream");
        let mut ws_sink = ws_sink_clone.lock().await;
        ws_sink.send(Message::Text(message.to_string())).await?;
        Ok(())
    }

    // ============================================
    //            Signaling Loop
    // ============================================

    pub async fn signaling_loop(
        &mut self, signaling_url: &str,
        peer_id: &str,
        initial_groups: Vec<String>
    ) -> Result<(), Box<dyn std::error::Error>> {
        // The connect_async simply connects to a given url which in this case is of ws:// type
        let (ws_stream, _) = connect_async(signaling_url).await
            .expect("Failed to connect");
        let (ws_sink, mut ws_stream) = ws_stream.split();

        let ws_sink = Arc::new(Mutex::new(ws_sink));
        self.ws_sink = Some(ws_sink.clone());

        {
            let mut sink = ws_sink.lock().await;
            sink.send(Message::Text(
                format!("register:{}:{}", peer_id, initial_groups.join(",")))
            ).await?;
        }

        // Register with the signaling server

        let (signaling_sender, mut signaling_receiver) = mpsc::channel(100);
        let ws_sink_clone = Arc::clone(&ws_sink);

        // handle outgoing signaling messages
        tokio::spawn(async move {
            while let Some(message) = signaling_receiver.next().await{
                let mut sink = ws_sink_clone.lock().await;
                if let Err(e) = sink.send(message).await {
                    log::log_message(&format!("Error sending message: {}", e));
                }
            }
        });

        // Receive initial list of peers in the channel
        if let Some(Ok(Message::Text(peer_list))) = ws_stream.next().await {
            let peers: Vec<String> = peer_list.split(',')
                .map(|s| s.to_string())
                .collect();
            // Create an offer for each peer and send it
            for peer in peers {
                let peer_connection = create_peer_connection(
                    &self.api,
                    &mut self.audio_data_channels,
                    signaling_sender.clone(),
                    peer_id.to_string(),
                    peer.clone().to_string(),
                    initial_groups.clone()
                ).await?;

                let offer_sdp = create_offer(&peer_connection).await?;
                self.peer_connections.insert(peer.clone(), peer_connection);

                let mut sink = ws_sink.lock().await;
                sink.send(Message::Text(
                    // Message Format
                    // {receiver}:{type}:{sender}:{message}
                    format!("{}:offer:{}:{}", peer, peer_id, offer_sdp)
                )).await?;
            }
        }
        // Recieve messages from the signaling server
        while let Some(message) = ws_stream.next().await{
            match message? {
                Message::Text(text) => {
                    if text.starts_with("new_peer:") {
                        let parts: Vec<&str> = text.splitn(3, ':').collect();
                        let new_peer_id = parts[1].to_string();
                        let new_peer_groups: Vec<String> = parts[2].split(',')
                            .map(|s| s.to_string()).collect();
                        self.peer_groups.insert(
                            new_peer_id.clone(),
                            new_peer_groups.clone()
                        );

                        let peer_connection = create_peer_connection(
                            &self.api,
                            &mut self.audio_data_channels,
                            signaling_sender.clone(),
                            peer_id.to_string(),
                            new_peer_id.to_string(),
                            new_peer_groups.clone()
                        ).await?;

                        let offer_sdp = create_offer(&peer_connection).await?;
                        self.peer_connections.insert(
                            new_peer_id.to_string(),
                            peer_connection
                        );
                        let mut sink = ws_sink.lock().await;
                        sink.send(Message::Text(
                            format!("{}:offer:{}:{}",
                                new_peer_id,
                                peer_id,
                                offer_sdp)
                        )).await?;

                    } else {
                        let parts: Vec<&str> = text.splitn(4, ':').collect();
                        let message_type = parts[0];
                        let remote_peer_id = parts[1];

                        match message_type {
                            "offer" => {
                                let sdp = parts[2];
                                let groups: Vec<String> = parts[3].split(',')
                                .map(|s| s.to_string()).collect();

                                let peer_connection = create_peer_connection(
                                    &self.api,
                                    &mut self.audio_data_channels,
                                    signaling_sender.clone(),
                                    peer_id.to_string(),
                                    remote_peer_id.to_string(),
                                    groups.clone()
                                ).await?;

                                set_remote_description(
                                    &peer_connection,
                                    RTCSessionDescription::offer(
                                        sdp.to_string()
                                    )?
                                ).await?;

                                let answer_sdp = create_answer(&peer_connection).await?;
                                self.peer_connections.insert(
                                    remote_peer_id.to_string(),
                                    peer_connection
                                );
                                let mut sink = ws_sink.lock().await;
                                sink.send(Message::Text(
                                    format!("{}:answer:{}:{}",
                                        remote_peer_id,
                                        peer_id,
                                        answer_sdp)
                                )).await?;
                            }
                            "answer" => {
                                let sdp = parts[2];

                                if let Some(peer_connection) = self.peer_connections
                                    .get_mut(remote_peer_id) {
                                    set_remote_description(
                                        &peer_connection,
                                        RTCSessionDescription::answer(
                                            sdp.to_string()
                                        )?
                                    ).await?;
                                }
                            }
                            "candidate" => {
                                let candidate = parts[2];
                                let ice_candidate = RTCIceCandidateInit {
                                    candidate: candidate.to_string(),
                                    sdp_mid: None,
                                    sdp_mline_index: None,
                                    username_fragment: None,
                                };
                                // Add ICE Candidate to the peer_connection
                                if let Some(peer_connection) = self.peer_connections
                                    .get_mut(remote_peer_id) {
                                    add_ice_candidate(
                                        peer_connection,
                                        ice_candidate
                                    ).await?;
                                }
                            }
                            "group_update:" => {
                                let new_groups: Vec<String> = parts[2].split(',')
                                    .map(|s| s.to_string()).collect();
                                self.peer_groups.insert(
                                    remote_peer_id.clone().to_string(), new_groups.clone()
                                );
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

    // ============================================
    //            Audio Handling
    // ============================================

    pub async fn send_audio(
        &self,
        audio_data: FormattedAudio,
        group: &str)
    -> Result<(), Box<dyn std::error::Error>>{
        let active = *self.audio_sending_active.lock().await;
        if !active {
            return Ok(());
        }
        // Check if the audio data is valid
        if let Ok(data) = audio_data {
            let bytes = Bytes::from(data);
            // Send audio to the specified destination using WebRTC
            if let Some(data_channels) = self.audio_data_channels.get(group) {
                for data_channel in data_channels {
                    if let Ok(_) = data_channel.send(&bytes).await {
                        log::log_message("Audio data sent successfully");
                    } else {
                        log::log_message("Failed to send audio data");
                    }
                }
            }
        } else {
            log::log_message("Invalid audio data");
        }
        Ok(())
    }
    pub async fn receive_audio(&self, group: &str) -> mpsc::Receiver<Vec<u8>> {
        let (sender, receiver) = mpsc::channel(100); // Channel to send audio data

        if let Some(data_channels) = self.audio_data_channels.get(group) {
            for data_channel in data_channels {
                let sender_clone = sender.clone();
                let data_channel_clone = Arc::clone(data_channel);
                let active = self.audio_receiving_active.clone();

                data_channel_clone.on_message(Box::new(move |msg: DataChannelMessage| {
                    let mut sender_clone = sender_clone.clone();
                    let active = active.clone();
                    tokio::spawn(async move {
                        let active = *active.lock().await;
                        if !active {
                            return;
                        }
                        let data = msg.data.to_vec();
                        // Send the data through the channel
                        if let Err(_) = sender_clone.try_send(data) {
                            log::log_message("Failed to send received audio data");
                        }
                    });
                    Box::pin(async {})
                }));
            }
        }
        receiver
    }
    // ============================================
    //            Control Audio Sending/Receiving
    // ============================================
    pub async fn stop_sending_audio(&self) {
        let mut active = self.audio_sending_active.lock().await;
        *active = false;
    }
    pub async fn resume_sending_audio(&self) {
        let mut active = self.audio_sending_active.lock().await;
        *active = true;
    }
    pub async fn stop_receiving_audio(&self) {
        let mut active = self.audio_receiving_active.lock().await;
        *active = false;
    }
    pub async fn resume_receiving_audio(&self) {
        let mut active = self.audio_receiving_active.lock().await;
        *active = true;
    }
}
// ============================================
//            Helper Functions
// ============================================
async fn create_peer_connection(
    api: &webrtc::api::API,
    audio_data_channels: &mut HashMap<String, Vec<Arc<RTCDataChannel>>>,
    signaling_sender: mpsc::Sender<Message>,
    peer_id: String,
    remote_peer_id: String,
    groups: Vec<String>,
) -> Result<RTCPeerConnection, Error> {
    let config = create_rtc_configuration();
    let peer_connection = api.new_peer_connection(config).await?;

    // Handle ICE candidates
    let remote_peer_id_clone = remote_peer_id.clone();
    let peer_id_clone = peer_id.clone();
    let signaling_sender_clone = signaling_sender.clone();
    peer_connection.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
        if let Some(candidate) = candidate {
            let candidate_string = candidate.to_string();
            let message = Message::Text(
                format!("{}:candidate:{}:{}",
                    remote_peer_id_clone,
                    peer_id_clone,
                    candidate_string
                )
            );
            let _ = signaling_sender_clone.clone().try_send(message);
        }
        Box::pin(async {})
    }));

    // Create a data channel for audio
    let data_channel_init = RTCDataChannelInit {
        ordered: Some(true),
        ..Default::default()
    };
    let audio_data_channel = peer_connection.create_data_channel("audio", Some(data_channel_init)).await?;
    for group in &groups {
        audio_data_channels.entry(group.to_string())
            .or_insert(Vec::new())
            .push(audio_data_channel.clone());
    }

    Ok(peer_connection)
}
// Media Engine
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
// API Creation
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
// RTC Configuration
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
// Create Offer
async fn create_offer(peer_connection: &RTCPeerConnection) -> Result<String, Error> {
    // First step when communicating with peer
    let offer = peer_connection.create_offer(None).await?;

    peer_connection.set_local_description(offer.clone()).await?;

    Ok(offer.sdp)
}
// Create Answer
async fn create_answer(peer_connection: &RTCPeerConnection) -> Result<String, Error> {
    // Response to peer upon receiving offer (sends answer)
    let answer = peer_connection.create_answer(None).await?;

    peer_connection.set_local_description(answer.clone()).await?;

    Ok(answer.sdp)
}
// Set Remote Description
async fn set_remote_description(peer_connection: &RTCPeerConnection, received_offer: RTCSessionDescription) -> Result<(), Error> {
    // Receives offer and sets it as the remote description
    peer_connection.set_remote_description(received_offer).await?;
    Ok(())
}
// Add ICE Candidate
async fn add_ice_candidate(peer_connection: &RTCPeerConnection, candidate: RTCIceCandidateInit) -> Result<(), Error> {
    // Adds Received ICE candidate
    peer_connection.add_ice_candidate(candidate).await?;
    Ok(())
}
// Handle Peer Connection Events
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
