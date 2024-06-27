// ============================================
//                  Imports
// ============================================
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::accept_async;
#[allow(unused_imports)]
use futures::{StreamExt, SinkExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::log;
use futures::stream::SplitSink;

type PeerMap = Arc<Mutex<HashMap<String, Arc<Mutex<SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, Message>>>>>>;
// ============================================
//                 Structures
// ============================================
pub struct WebSocketStream {
    peer_map: PeerMap,
}
// ============================================
//              Implementation
// ============================================
impl WebSocketStream {
    // Creates a new WebSocketStream instance with an empty peer map
    pub fn new() -> Self {
        Self {
            peer_map: PeerMap::default(),
        }
    }

    // ============================================
    //            Start WebSocket Server
    // ============================================

    // Starts the WebSocket server and listens for incoming connections on the specified address
    pub async fn start(&self, addr: &str) {
        let listener = TcpListener::bind(addr).await.expect("Failed to bind");
        log::log_message(&format!("WebSocket server listening on {}", addr));

        // Continuously accept incoming connections and handle them concurrently
        while let Ok((stream, _addr)) = listener.accept().await {
            let peer_map = self.peer_map.clone();
            tokio::spawn(handle_connection(peer_map, stream));
        }
    }
    // ============================================
    //              Relay Message
    // Relay a message to a specific peer
    // identified by the target peer ID.
    // ============================================

    pub async fn relay_message(&self, target_peer_id: &str,
        message_content: &str) -> Result<(), Box<dyn std::error::Error>> {

        let peers = self.peer_map.lock().await;
        // If the target peet exists, send the message to them
        if let Some(peer) = peers.get(target_peer_id) {
            let mut peer = peer.lock().await;
            peer.send(Message::Text(message_content.to_string())).await?;
        }
        Ok(())
    }
    // ============================================
    //            Get Peer List
    // ============================================
     pub async fn get_peer_list(&self) -> Vec<String> {
        let peers = self.peer_map.lock().await;
        peers.keys().cloned().collect::<Vec<String>>()
    }
}
// ============================================
//            Handle Connection
// ============================================
async fn handle_connection(peer_map: PeerMap, raw_stream: tokio::net::TcpStream) {
    // Accept the WebSocket Connection
    let ws_stream = accept_async(raw_stream).await.expect("Failed to accept");
    // Split the WebSocket
    #[allow(unused_mut)]
    let (mut write, mut read) = ws_stream.split();
    let write = Arc::new(Mutex::new(write));

    // Continuously read messages from the stream
    while let Some(result) = read.next().await {
        match result {
            Ok(message) => {
                match message {
                    // Checks if it's a text message
                    Message::Text(text) => {
                        // Register a peer
                        if text.starts_with("register:") {
                            let parts: Vec<&str> = text.splitn(3, ':').collect();
                            let peer_id = parts[1].to_string();
                            let _groups: Vec<String> = parts[2].split(',').
                                map(|s| s.to_string()).collect();

                            let mut peers = peer_map.lock().await;
                            peers.insert(peer_id.clone(), write.clone());

                            // Send the list of peers to the newly connected peer
                            let peer_list = peers.keys().cloned()
                                .collect::<Vec<String>>().join(",");
                            let new_peer = peers.get(&peer_id).unwrap();
                            let mut new_peer = new_peer.lock().await;
                            new_peer.send(Message::Text(peer_list)).await
                                .expect("Failed to send peer list");

                            // Notify existing peers about the new peer
                            for(id, peer) in peers.iter() {
                                if id != &peer_id {
                                    let mut peer = peer.lock().await;
                                    peer.send(Message::Text(
                                        format!("new_peer:{}:{}", peer_id, parts[2]))).await.expect("Failed to notify peers about new peer");
                                }
                            }

                        } else {
                            let parts: Vec<&str> = text.splitn(4, ":").collect();
                            if parts.len() == 4 {
                                let target_peer_id = parts[0];
                                let message_content = format!("{}:{}:{}", parts[1], parts[2], parts[3]);

                                let peers = peer_map.lock().await;
                                if let Some(peer) = peers.get(target_peer_id) {
                                    let mut peer = peer.lock().await;
                                    peer.send(Message::Text(message_content.to_string())).await.expect("Failed to send message");
                                }
                            }
                        }
                    }
                    _ => {}
                }
            },
            Err(err) => {
                log::log_message(&format!("Error receiving message: {}", err));
                break;
            },
        }
    }
}
