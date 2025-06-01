use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use snow::{Builder, HandshakeState, TransportState, Keypair};
use std::error::Error;
use rand::Rng;

const NOISE_PATTERN: &str = "Noise_XXpsk2_25519_AESGCM_SHA256";
const PSK: &[u8; 32] = b"my_super_secret_pre_shared_key!!";
const MIN_REKEY_INTERVAL_SECS: u64 = 30; // Minimum 30 seconds between rekeys
const MAX_REKEY_INTERVAL_SECS: u64 = 120; // Maximum 120 seconds between rekeys

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    sender: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ControlMessage {
    #[serde(rename = "chat")]
    Chat { sender: String, content: String },
    #[serde(rename = "rekey")]
    Rekey,
}

#[derive(Debug)]
enum NoiseError {
    HandshakeError(String),
    EncryptionError(String),
    DecryptionError(String),
}

impl std::fmt::Display for NoiseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NoiseError::HandshakeError(msg) => write!(f, "Handshake error: {}", msg),
            NoiseError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            NoiseError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
        }
    }
}

impl Error for NoiseError {}

struct NoiseSession {
    transport: TransportState,
    message_count: u64,
    next_rekey_time: Instant,
    rekey_count: u64,
}

impl NoiseSession {
    fn new(transport: TransportState) -> Self {
        let mut rng = rand::thread_rng();
        let initial_interval = rng.gen_range(MIN_REKEY_INTERVAL_SECS..=MAX_REKEY_INTERVAL_SECS);
        let next_rekey_time = Instant::now() + Duration::from_secs(initial_interval);
        
        println!("🎲 Random rekey interval set: {} seconds", initial_interval);
        println!("⏰ Next rekey scheduled for: {:?}", next_rekey_time);
        
        Self {
            transport,
            message_count: 0,
            next_rekey_time,
            rekey_count: 0,
        }
    }

    fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, NoiseError> {
        let mut ciphertext = vec![0u8; plaintext.len() + 16]; // +16 for AEAD tag
        let len = self
            .transport
            .write_message(plaintext, &mut ciphertext)
            .map_err(|e| NoiseError::EncryptionError(e.to_string()))?;
        ciphertext.truncate(len);
        
        self.message_count += 1;
        Ok(ciphertext)
    }

    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, NoiseError> {
        let mut plaintext = vec![0u8; ciphertext.len()];
        let len = self
            .transport
            .read_message(ciphertext, &mut plaintext)
            .map_err(|e| NoiseError::DecryptionError(e.to_string()))?;
        plaintext.truncate(len);
        
        self.message_count += 1;
        Ok(plaintext)
    }

    fn perform_rekey(&mut self) {
        self.rekey_count += 1;
        println!("🔄 Server performing time-based key rotation #{} (total messages: {})", 
            self.rekey_count, self.message_count);
        self.transport.rekey_outgoing();
        self.transport.rekey_incoming();
        
        // Generate new random interval for next rekey
        let mut rng = rand::thread_rng();
        let next_interval = rng.gen_range(MIN_REKEY_INTERVAL_SECS..=MAX_REKEY_INTERVAL_SECS);
        self.next_rekey_time = Instant::now() + Duration::from_secs(next_interval);
        
        println!("✅ Key rotation completed - next rekey in {} seconds", next_interval);
        println!("⏰ Next rekey scheduled for: {:?}", self.next_rekey_time);
    }

    fn should_rekey(&self) -> bool {
        Instant::now() >= self.next_rekey_time
    }

    fn get_message_count(&self) -> u64 {
        self.message_count
    }

    fn get_rekey_count(&self) -> u64 {
        self.rekey_count
    }

    fn get_time_until_next_rekey(&self) -> Duration {
        self.next_rekey_time.saturating_duration_since(Instant::now())
    }
}

fn create_responder() -> Result<HandshakeState, NoiseError> {
    let builder = Builder::new(NOISE_PATTERN.parse().unwrap());
    let keypair = builder.generate_keypair().map_err(|e| NoiseError::HandshakeError(e.to_string()))?;
    
    builder
        .local_private_key(&keypair.private)
        .psk(2, PSK)
        .build_responder()
        .map_err(|e| NoiseError::HandshakeError(e.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await?;
    println!("🚀 Secure WebSocket server listening on: {}", addr);
    println!("🔐 Using Noise protocol: {}", NOISE_PATTERN);
    println!("⏰ Time-based random rekeying enabled: {}-{} seconds", MIN_REKEY_INTERVAL_SECS, MAX_REKEY_INTERVAL_SECS);

    // For simplicity, handle one connection at a time
    if let Ok((stream, addr)) = listener.accept().await {
        println!("📱 New connection from: {}", addr);
        handle_connection(stream).await;
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(err) => {
            eprintln!("❌ Failed to accept WebSocket: {}", err);
            return;
        }
    };

    println!("✅ WebSocket connection established!");
    println!("🤝 Starting Noise handshake...");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Perform Noise handshake
    let noise_session = match perform_noise_handshake_responder(&mut ws_sender, &mut ws_receiver).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("❌ Noise handshake failed: {}", e);
            return;
        }
    };

    println!("🔐 Secure channel established!");
    println!("💬 Type messages to send to client:");

    let noise_session = Arc::new(Mutex::new(noise_session));
    let noise_session_clone = Arc::clone(&noise_session);
    let noise_session_timer = Arc::clone(&noise_session);

    // Background timer task for automatic rekeying
    let ws_sender_clone = Arc::new(Mutex::new(ws_sender));
    let ws_sender_timer = Arc::clone(&ws_sender_clone);
    
    let timer_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5)); // Check every 5 seconds
        
        loop {
            interval.tick().await;
            
            let should_rekey = {
                let session = noise_session_timer.lock().await;
                session.should_rekey()
            };
            
            if should_rekey {
                println!("⏰ Time-based rekey triggered!");
                
                // Send rekey command to client
                let rekey_msg = ControlMessage::Rekey;
                match serde_json::to_string(&rekey_msg) {
                    Ok(json) => {
                        let mut session = noise_session_timer.lock().await;
                        match session.encrypt(json.as_bytes()) {
                            Ok(encrypted) => {
                                let mut sender = ws_sender_timer.lock().await;
                                if let Err(err) = sender.send(Message::Binary(encrypted)).await {
                                    eprintln!("❌ Failed to send rekey command: {}", err);
                                    break;
                                }
                                // Perform rekey on server side
                                session.perform_rekey();
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to encrypt rekey command: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to serialize rekey command: {}", e);
                        break;
                    }
                }
            }
        }
    });

    // Handle incoming encrypted messages from client
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(encrypted_data)) => {
                    let mut session = noise_session_clone.lock().await;
                    match session.decrypt(&encrypted_data) {
                        Ok(decrypted) => {
                            match String::from_utf8(decrypted) {
                                Ok(json_str) => {
                                    match serde_json::from_str::<ControlMessage>(&json_str) {
                                        Ok(ControlMessage::Chat { sender, content }) => {
                                            println!("📨 {}: {}", sender, content);
                                            // Show status occasionally
                                            let count = session.get_message_count();
                                            if count % 10 == 0 {
                                                let time_left = session.get_time_until_next_rekey();
                                                println!("📊 Messages: {} | Rekeys: {} | Next rekey in: {}s", 
                                                    count, session.get_rekey_count(), time_left.as_secs());
                                            }
                                        }
                                        Ok(ControlMessage::Rekey) => {
                                            println!("🔄 Received rekey acknowledgment from client");
                                        }
                                        Err(_) => {
                                            println!("📨 Received: {}", json_str);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to decode UTF-8: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to decrypt message: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("🔌 Client disconnected");
                    let session = noise_session_clone.lock().await;
                    println!("📊 Final stats - Messages: {} | Rekeys: {}", 
                        session.get_message_count(), session.get_rekey_count());
                    break;
                }
                _ => {} // Ignore other message types
            }
        }
    });

    // Handle server input and send encrypted messages to client
    let input_task = tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        print!("> ");
        io::stdout().flush().unwrap();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();

            if line.is_empty() {
                print!("> ");
                io::stdout().flush().unwrap();
                continue;
            }

            if line.eq_ignore_ascii_case("quit") {
                break;
            }

            if line.eq_ignore_ascii_case("status") {
                let session = noise_session.lock().await;
                let time_left = session.get_time_until_next_rekey();
                println!("📊 Status - Messages: {} | Rekeys: {} | Next rekey in: {}s", 
                    session.get_message_count(), session.get_rekey_count(), time_left.as_secs());
                print!("> ");
                io::stdout().flush().unwrap();
                continue;
            }

            let chat_msg = ControlMessage::Chat {
                sender: "Server".to_string(),
                content: line.to_string(),
            };

            match serde_json::to_string(&chat_msg) {
                Ok(json) => {
                    let mut session = noise_session.lock().await;
                    match session.encrypt(json.as_bytes()) {
                        Ok(encrypted) => {
                            let mut sender = ws_sender_clone.lock().await;
                            if let Err(err) = sender.send(Message::Binary(encrypted)).await {
                                eprintln!("❌ Failed to send message: {}", err);
                                break;
                            }
                            println!("📤 You: {}", line);
                            
                            // Show status occasionally
                            let count = session.get_message_count();
                            if count % 10 == 0 {
                                let time_left = session.get_time_until_next_rekey();
                                println!("📊 Messages: {} | Rekeys: {} | Next rekey in: {}s", 
                                    count, session.get_rekey_count(), time_left.as_secs());
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to encrypt message: {}", e);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("❌ Failed to serialize message: {}", err);
                }
            }

            print!("> ");
            io::stdout().flush().unwrap();
        }
    });

    // Wait for any task to complete
    tokio::select! {
        _ = timer_task => {}
        _ = incoming_task => {}
        _ = input_task => {}
    }

    println!("🚪 Connection closed");
}

async fn perform_noise_handshake_responder(
    ws_sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
    ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
) -> Result<NoiseSession, Box<dyn std::error::Error>> {
    let mut handshake = create_responder()?;

    // Wait for first message from client
    if let Some(msg) = ws_receiver.next().await {
        match msg? {
            Message::Binary(data) => {
                let mut buf = vec![0u8; 65535];
                handshake.read_message(&data, &mut buf)?;
                
                let len = handshake.write_message(&[], &mut buf)?;
                let response = buf[..len].to_vec();
                
                // Send response
                ws_sender.send(Message::Binary(response)).await?;

                if let Some(msg) = ws_receiver.next().await {
                    match msg? {
                        Message::Binary(data) => {
                            handshake.read_message(&data, &mut buf)?;
                            
                            let transport = handshake.into_transport_mode()?;
                            Ok(NoiseSession::new(transport))
                        }
                        _ => Err("Expected binary message for handshake".into()),
                    }
                } else {
                    Err("Connection closed during handshake".into())
                }
            }
            _ => Err("Expected binary message for handshake".into()),
        }
    } else {
        Err("Connection closed during handshake".into())
    }
} 