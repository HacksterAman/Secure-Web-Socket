use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use snow::{Builder, HandshakeState, TransportState};
use std::error::Error;

const NOISE_PATTERN: &str = "Noise_XXpsk2_25519_AESGCM_SHA256";
const PSK: &[u8; 32] = b"my_super_secret_pre_shared_key!!";

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
    println!("Connecting to server at: {}", url);

    let (ws_stream, _) = connect_async(url).await?;
    println!("Connected to server");
    println!("Starting Noise handshake...");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let noise_session = match perform_noise_handshake_initiator(&mut ws_sender, &mut ws_receiver).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("Noise handshake failed: {}", e);
            return Ok(());
        }
    };

    println!("Secure channel established");

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
                            eprintln!("Decryption failed: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("Server disconnected");
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
                println!("Disconnecting...");
                let _ = ws_sender.send(Message::Close(None)).await;
                break;
            }

            let chat_msg = ChatMessage {
                sender: String::new(),
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

            print!("> ");
            io::stdout().flush().unwrap();
        }
    });

    tokio::select! {
        _ = incoming_task => {}
        _ = input_task => {}
    }

    println!("Disconnected");
    Ok(())
}

async fn perform_noise_handshake_initiator(
    ws_sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
    ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
) -> Result<NoiseSession, Box<dyn std::error::Error>> {
    let mut handshake = create_initiator()?;
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