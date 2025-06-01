use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use snow::{Builder, HandshakeState, TransportState, Keypair};
use std::error::Error;

const NOISE_PATTERN: &str = "Noise_XXpsk2_25519_AESGCM_SHA256";
const PSK: &[u8; 32] = b"my_super_secret_pre_shared_key!!";

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
    rekey_count: u64,
}

impl NoiseSession {
    fn new(transport: TransportState) -> Self {
        Self {
            transport,
            message_count: 0,
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
        println!("🔄 Client performing time-based key rotation #{} (total messages: {})", 
            self.rekey_count, self.message_count);
        self.transport.rekey_incoming();
        self.transport.rekey_outgoing();
        println!("✅ Client key rotation completed - synchronized with server");
    }

    fn get_message_count(&self) -> u64 {
        self.message_count
    }

    fn get_rekey_count(&self) -> u64 {
        self.rekey_count
    }
}

fn create_initiator() -> Result<HandshakeState, NoiseError> {
    let builder = Builder::new(NOISE_PATTERN.parse().unwrap());
    let keypair = builder.generate_keypair().map_err(|e| NoiseError::HandshakeError(e.to_string()))?;
    
    builder
        .local_private_key(&keypair.private)
        .psk(2, PSK)
        .build_initiator()
        .map_err(|e| NoiseError::HandshakeError(e.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://127.0.0.1:8080";
    println!("🔗 Connecting to WebSocket server at: {}", url);
    println!("⏰ Server-controlled time-based random rekeying enabled");

    let (ws_stream, _) = connect_async(url).await?;
    println!("✅ Connected to server!");
    println!("🤝 Starting Noise handshake...");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Perform Noise handshake
    let noise_session = match perform_noise_handshake_initiator(&mut ws_sender, &mut ws_receiver).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("❌ Noise handshake failed: {}", e);
            return Ok(());
        }
    };

    println!("🔐 Secure channel established!");
    println!("💬 Type messages to send to server (or 'status' for info):");

    let noise_session = Arc::new(Mutex::new(noise_session));
    let noise_session_clone = Arc::clone(&noise_session);

    // Handle incoming encrypted messages from server
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
                                                println!("📊 Messages: {} | Rekeys: {}", 
                                                    count, session.get_rekey_count());
                                            }
                                        }
                                        Ok(ControlMessage::Rekey) => {
                                            println!("⏰ Received time-based rekey command from server");
                                            session.perform_rekey();
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
                    println!("🔌 Server disconnected");
                    let session = noise_session_clone.lock().await;
                    println!("📊 Final stats - Messages: {} | Rekeys: {}", 
                        session.get_message_count(), session.get_rekey_count());
                    break;
                }
                _ => {} // Ignore other message types
            }
        }
    });

    // Handle user input and send encrypted messages
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
                println!("👋 Disconnecting...");
                if let Err(err) = ws_sender.send(Message::Close(None)).await {
                    eprintln!("❌ Failed to send close message: {}", err);
                }
                break;
            }

            if line.eq_ignore_ascii_case("status") {
                let session = noise_session.lock().await;
                println!("📊 Status - Messages: {} | Rekeys: {}", 
                    session.get_message_count(), session.get_rekey_count());
                print!("> ");
                io::stdout().flush().unwrap();
                continue;
            }

            let chat_msg = ControlMessage::Chat {
                sender: "Client".to_string(),
                content: line.to_string(),
            };

            match serde_json::to_string(&chat_msg) {
                Ok(json) => {
                    let mut session = noise_session.lock().await;
                    match session.encrypt(json.as_bytes()) {
                        Ok(encrypted) => {
                            if let Err(err) = ws_sender.send(Message::Binary(encrypted)).await {
                                eprintln!("❌ Failed to send message: {}", err);
                                break;
                            }
                            println!("📤 You: {}", line);
                            
                            // Show status occasionally
                            let count = session.get_message_count();
                            if count % 10 == 0 {
                                println!("📊 Messages: {} | Rekeys: {}", 
                                    count, session.get_rekey_count());
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

    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {}
        _ = input_task => {}
    }

    println!("🚪 Disconnected");
    Ok(())
}

async fn perform_noise_handshake_initiator(
    ws_sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
    ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
) -> Result<NoiseSession, Box<dyn std::error::Error>> {
    let mut handshake = create_initiator()?;
    let mut buf = vec![0u8; 65535];

    // -> e
    let len = handshake.write_message(&[], &mut buf)?;
    let message1 = buf[..len].to_vec();
    
    // Send first message
    ws_sender.send(Message::Binary(message1)).await?;

    // Wait for response
    if let Some(msg) = ws_receiver.next().await {
        match msg? {
            Message::Binary(data) => {
                // <- e, ee, s, es
                handshake.read_message(&data, &mut buf)?;
                
                // -> s, se
                let len = handshake.write_message(&[], &mut buf)?;
                let final_message = buf[..len].to_vec();
                
                // Send final message
                ws_sender.send(Message::Binary(final_message)).await?;
                
                let transport = handshake.into_transport_mode()?;
                Ok(NoiseSession::new(transport))
            }
            _ => Err("Expected binary message for handshake".into()),
        }
    } else {
        Err("Connection closed during handshake".into())
    }
} 