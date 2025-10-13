use std::sync::Arc;
use std::collections::HashMap;
use std::io::{self, Write};
use tokio::sync::{Mutex, broadcast};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use snow::{Builder, HandshakeState, TransportState};
use std::error::Error;

const NOISE_PATTERN: &str = "Noise_XXpsk2_25519_AESGCM_SHA256";
const PSK: &[u8; 32] = b"my_super_secret_pre_shared_key!!";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    sender: String,
    content: String,
}

#[derive(Debug, Clone)]
struct ServerCommand {
    target: Option<String>,  // None = broadcast, Some(name) = send to specific client
    message: ChatMessage,
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

fn create_responder() -> Result<HandshakeState, NoiseError> {
    let builder = Builder::new(NOISE_PATTERN.parse().unwrap());
    let keypair = builder.generate_keypair().map_err(|e| NoiseError::HandshakeError(e.to_string()))?;
    
    builder
        .local_private_key(&keypair.private)
        .psk(2, PSK)
        .build_responder()
        .map_err(|e| NoiseError::HandshakeError(e.to_string()))
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await?;
    println!("Server listening on: {}", addr);
    println!("Using Noise protocol: {}", NOISE_PATTERN);
    println!("Commands: '@ClientName message' to send to specific client, or 'message' to broadcast");

    let (broadcast_tx, _) = broadcast::channel::<ChatMessage>(100);
    let (server_cmd_tx, _) = broadcast::channel::<ServerCommand>(100);
    let clients = Arc::new(Mutex::new(HashMap::new()));
    let client_counter = Arc::new(Mutex::new(0u32));

    // Server input task
    let server_cmd_tx_clone = server_cmd_tx.clone();
    let clients_clone = clients.clone();
    tokio::spawn(async move {
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

            let (target, content) = if line.starts_with('@') {
                // Targeted message: @ClientName message
                if let Some(space_pos) = line.find(' ') {
                    let name = &line[1..space_pos];
                    let msg = &line[space_pos + 1..];
                    (Some(name.to_string()), msg.to_string())
                } else {
                    println!("Invalid format. Use: @ClientName message");
                    print!("> ");
                    io::stdout().flush().unwrap();
                    continue;
                }
            } else {
                // Broadcast message
                (None, line.to_string())
            };

            let cmd = ServerCommand {
                target: target.clone(),
                message: ChatMessage {
                    sender: "Server".to_string(),
                    content: content.clone(),
                },
            };

            if let Some(name) = &target {
                let clients_map = clients_clone.lock().await;
                if clients_map.values().any(|n| n == name) {
                    println!("To {}: {}", name, content);
                } else {
                    println!("Client '{}' not found", name);
                    print!("> ");
                    io::stdout().flush().unwrap();
                    continue;
                }
            } else {
                println!("Broadcast: {}", content);
            }

            let _ = server_cmd_tx_clone.send(cmd);
            print!("> ");
            io::stdout().flush().unwrap();
        }
    });

    loop {
        if let Ok((stream, addr)) = listener.accept().await {
            println!("New connection from: {}", addr);
            let broadcast_tx = broadcast_tx.clone();
            let server_cmd_tx = server_cmd_tx.clone();
            let clients = clients.clone();
            let client_counter = client_counter.clone();
            
            tokio::spawn(async move {
                handle_connection(stream, broadcast_tx, server_cmd_tx, clients, client_counter).await;
            });
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    broadcast_tx: broadcast::Sender<ChatMessage>,
    server_cmd_tx: broadcast::Sender<ServerCommand>,
    clients: Arc<Mutex<HashMap<u32, String>>>,
    client_counter: Arc<Mutex<u32>>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(err) => {
            eprintln!("Failed to accept WebSocket: {}", err);
            return;
        }
    };

    println!("WebSocket connection established");
    println!("Starting Noise handshake...");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let noise_session = match perform_noise_handshake_responder(&mut ws_sender, &mut ws_receiver).await {
        Ok(session) => session,
        Err(e) => {
            eprintln!("Noise handshake failed: {}", e);
            return;
        }
    };

    println!("Secure channel established");

    let noise_session = Arc::new(Mutex::new(noise_session));
    
    // Request client name
    let name_request = ChatMessage {
        sender: "Server".to_string(),
        content: "Please enter your name:".to_string(),
    };
    
    match serde_json::to_string(&name_request) {
        Ok(json) => {
            let mut session = noise_session.lock().await;
            if let Ok(encrypted) = session.encrypt(json.as_bytes()) {
                if let Err(err) = ws_sender.send(Message::Binary(encrypted)).await {
                    eprintln!("Failed to request name: {}", err);
                    return;
                }
            }
        }
        Err(_) => return,
    }

    // Wait for client name
    let client_name = if let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Binary(encrypted_data)) => {
                let mut session = noise_session.lock().await;
                match session.decrypt(&encrypted_data) {
                    Ok(decrypted) => {
                        match String::from_utf8(decrypted) {
                            Ok(json_str) => {
                                if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&json_str) {
                                    chat_msg.content
                                } else {
                                    return;
                                }
                            }
                            Err(_) => return,
                        }
                    }
                    Err(_) => return,
                }
            }
            _ => return,
        }
    } else {
        return;
    };

    let client_id = {
        let mut counter = client_counter.lock().await;
        *counter += 1;
        *counter
    };

    clients.lock().await.insert(client_id, client_name.clone());
    println!("{} joined the chat", client_name);

    // let join_msg = ChatMessage {
    //     sender: "Server".to_string(),
    //     content: format!("{} joined the chat", client_name),
    // };
    // let _ = broadcast_tx.send(join_msg);

    let mut broadcast_rx = broadcast_tx.subscribe();
    let mut server_cmd_rx = server_cmd_tx.subscribe();
    let noise_session_recv = Arc::clone(&noise_session);
    let ws_sender = Arc::new(Mutex::new(ws_sender));
    let ws_sender_broadcast = Arc::clone(&ws_sender);
    let ws_sender_server = Arc::clone(&ws_sender);
    let noise_session_server = Arc::clone(&noise_session);
    let client_name_clone = client_name.clone();
    let client_name_server = client_name.clone();

    // Broadcast messages to this client
    let broadcast_task = tokio::spawn(async move {
        while let Ok(chat_msg) = broadcast_rx.recv().await {
            if chat_msg.sender != client_name_clone {
                if let Ok(json) = serde_json::to_string(&chat_msg) {
                    let mut session = noise_session_recv.lock().await;
                    if let Ok(encrypted) = session.encrypt(json.as_bytes()) {
                        let mut sender = ws_sender_broadcast.lock().await;
                        if sender.send(Message::Binary(encrypted)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Server commands to this client
    let server_cmd_task = tokio::spawn(async move {
        while let Ok(cmd) = server_cmd_rx.recv().await {
            // Check if message is for this client or broadcast
            let should_send = match &cmd.target {
                None => true,  // Broadcast to all
                Some(target_name) => target_name == &client_name_server,  // Targeted message
            };

            if should_send {
                if let Ok(json) = serde_json::to_string(&cmd.message) {
                    let mut session = noise_session_server.lock().await;
                    if let Ok(encrypted) = session.encrypt(json.as_bytes()) {
                        let mut sender = ws_sender_server.lock().await;
                        if sender.send(Message::Binary(encrypted)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Receive messages from this client
    let noise_session_send = Arc::clone(&noise_session);
    let broadcast_tx_clone = broadcast_tx.clone();
    let client_name_send = client_name.clone();
    
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(encrypted_data)) => {
                    let mut session = noise_session_send.lock().await;
                    match session.decrypt(&encrypted_data) {
                        Ok(decrypted) => {
                            if let Ok(json_str) = String::from_utf8(decrypted) {
                                if let Ok(mut chat_msg) = serde_json::from_str::<ChatMessage>(&json_str) {
                                    chat_msg.sender = client_name_send.clone();
                                    println!("{}: {}", chat_msg.sender, chat_msg.content);
                                    let _ = broadcast_tx_clone.send(chat_msg);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Decryption failed: {}", e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("{} disconnected", client_name_send);
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = broadcast_task => {}
        _ = server_cmd_task => {}
        _ = receive_task => {}
    }

    clients.lock().await.remove(&client_id);
    let leave_msg = ChatMessage {
        sender: "Server".to_string(),
        content: format!("{} left the chat", client_name),
    };
    let _ = broadcast_tx.send(leave_msg);
}

async fn perform_noise_handshake_responder(
    ws_sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
    ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
) -> Result<NoiseSession, Box<dyn std::error::Error>> {
    let mut handshake = create_responder()?;
    let mut buf = vec![0u8; 65535];

    if let Some(msg) = ws_receiver.next().await {
        match msg? {
            Message::Binary(data) => {
                handshake.read_message(&data, &mut buf)?;
                let len = handshake.write_message(&[], &mut buf)?;
                ws_sender.send(Message::Binary(buf[..len].to_vec())).await?;

                if let Some(msg) = ws_receiver.next().await {
                    match msg? {
                        Message::Binary(data) => {
                            handshake.read_message(&data, &mut buf)?;
                            let transport = handshake.into_transport_mode()?;
                            Ok(NoiseSession::new(transport))
                        }
                        _ => Err("Expected binary message".into()),
                    }
                } else {
                    Err("Connection closed".into())
                }
            }
            _ => Err("Expected binary message".into()),
        }
    } else {
        Err("Connection closed".into())
    }
} 