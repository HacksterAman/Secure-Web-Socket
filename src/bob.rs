use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use snow::{Builder, HandshakeState, TransportState};
use std::error::Error;

mod qkd_module;
use qkd_module::{QkdConfig, QkdClient};

const NOISE_PATTERN: &str = "Noise_XXpsk2_25519_AESGCM_SHA256";

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    sender: String,
    content: String,
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
}

impl NoiseSession {
    fn new(transport: TransportState) -> Self {
        Self { transport }
    }

    fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, NoiseError> {
        let mut ciphertext = vec![0u8; plaintext.len() + 16];
        let len = self
            .transport
            .write_message(plaintext, &mut ciphertext)
            .map_err(|e| NoiseError::EncryptionError(e.to_string()))?;
        ciphertext.truncate(len);
        Ok(ciphertext)
    }

    fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, NoiseError> {
        let mut plaintext = vec![0u8; ciphertext.len()];
        let len = self
            .transport
            .read_message(ciphertext, &mut plaintext)
            .map_err(|e| NoiseError::DecryptionError(e.to_string()))?;
        plaintext.truncate(len);
        Ok(plaintext)
    }
}

fn create_initiator(psk: &[u8; 32]) -> Result<HandshakeState, NoiseError> {
    let builder = Builder::new(NOISE_PATTERN.parse().unwrap());
    let keypair = builder.generate_keypair().map_err(|e| NoiseError::HandshakeError(e.to_string()))?;
    
    builder
        .local_private_key(&keypair.private)
        .psk(2, psk)
        .build_initiator()
        .map_err(|e| NoiseError::HandshakeError(e.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load QKD configuration from TOML
    let config = QkdConfig::load("qkd_config.toml")?;

    // Create QKD client for Bob
    let qkd_client = QkdClient::new(
        config.bob.clone(),
        config.general.kme_url.clone(),
        config.general.key_size,
    );

    // Retrieve QKD key from KME API
    let qkd_key = qkd_client.get_key(&config.server.sae_id).await?;
    
    if qkd_key.len() != 32 {
        return Err("Invalid key size from QKD system".into());
    }

    // Use QKD key as PSK for Noise Protocol
    let mut psk = [0u8; 32];
    psk.copy_from_slice(&qkd_key);
    println!("[Bob] Using QKD key as PSK for Noise Protocol\n");

    // Connect to WebSocket server
    let url = format!("ws://{}", config.websocket.server_address);
    let (ws_stream, _) = connect_async(&url).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Exchange name with server
    if let Some(msg) = ws_receiver.next().await {
        match msg? {
            Message::Text(text) if text == "NAME_REQUEST" => {
                ws_sender.send(Message::Text("Bob".to_string())).await?;
            }
            _ => return Err("Expected NAME_REQUEST from server".into()),
        }
    } else {
        return Err("Connection closed during name exchange".into());
    }

    // Perform Noise handshake with QKD-derived PSK
    let noise_session = match perform_noise_handshake_initiator(&mut ws_sender, &mut ws_receiver, &psk).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("[Bob] Handshake failed: {}", e);
            return Ok(());
        }
    };

    println!("[Bob] Secure channel established. Ready to chat.\n");

    let noise_session = Arc::new(Mutex::new(noise_session));
    let noise_session_clone = Arc::clone(&noise_session);

    // Handle incoming messages
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(encrypted_data)) => {
                    let mut session = noise_session_clone.lock().await;
                    match session.decrypt(&encrypted_data) {
                        Ok(decrypted) => {
                            if let Ok(json_str) = String::from_utf8(decrypted) {
                                if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&json_str) {
                                    println!("{}: {}", chat_msg.sender, chat_msg.content);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[Bob] Decryption failed: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("[Bob] Server disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    // Handle user input
    let input_task = tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        print!("Bob> ");
        io::stdout().flush().unwrap();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();

            if line.is_empty() {
                print!("Bob> ");
                io::stdout().flush().unwrap();
                continue;
            }

            if line.eq_ignore_ascii_case("quit") {
                println!("[Bob] Disconnecting...");
                let _ = ws_sender.send(Message::Close(None)).await;
                break;
            }

            let chat_msg = ChatMessage {
                sender: "Bob".to_string(),
                content: line.to_string(),
            };

            if let Ok(json) = serde_json::to_string(&chat_msg) {
                let mut session = noise_session.lock().await;
                if let Ok(encrypted) = session.encrypt(json.as_bytes()) {
                    if ws_sender.send(Message::Binary(encrypted)).await.is_err() {
                        break;
                    }
                }
            }

            print!("Bob> ");
            io::stdout().flush().unwrap();
        }
    });

    tokio::select! {
        _ = incoming_task => {}
        _ = input_task => {}
    }

    println!("[Bob] Disconnected");
    Ok(())
}

async fn perform_noise_handshake_initiator(
    ws_sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
    ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    psk: &[u8; 32],
) -> Result<NoiseSession, Box<dyn std::error::Error>> {
    let mut handshake = create_initiator(psk)?;
    let mut buf = vec![0u8; 65535];

    let len = handshake.write_message(&[], &mut buf)?;
    ws_sender.send(Message::Binary(buf[..len].to_vec())).await?;

    if let Some(msg) = ws_receiver.next().await {
        match msg? {
            Message::Binary(data) => {
                handshake.read_message(&data, &mut buf)?;
                let len = handshake.write_message(&[], &mut buf)?;
                ws_sender.send(Message::Binary(buf[..len].to_vec())).await?;
                let transport = handshake.into_transport_mode()?;
                Ok(NoiseSession::new(transport))
            }
            _ => Err("Expected binary message".into()),
        }
    } else {
        Err("Connection closed".into())
    }
}

