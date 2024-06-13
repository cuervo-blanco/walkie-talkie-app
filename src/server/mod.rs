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

pub struct WebSocketStream {
    peer_map: PeerMap,
}

impl WebSocketStream {
    // Creates a new WebSocketStream instance with an empty peer map
    pub fn new() -> Self {
        Self {
            peer_map: PeerMap::default(),
        }
    }

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

    // Relay a message to a specific peer identified by the target peer ID
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
}

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
                            let peer_id = text[9..].to_string();
                            let mut peers = peer_map.lock().await;
                            peers.insert(peer_id, write.clone());
                        } else {
                            let parts: Vec<&str> = text.splitn(2, ":").collect();
                            if parts.len() == 2 {
                                let target_peer_id = parts[0];
                                let message_content = parts[1];

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
