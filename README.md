# Secure WebSocket Chat

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Security](https://img.shields.io/badge/Security-Noise%20Protocol-green.svg)](https://noiseprotocol.org)
[![WebSocket](https://img.shields.io/badge/WebSocket-Secure-blue.svg)](https://tools.ietf.org/html/rfc6455)

A secure multi-client chat application built with Rust using WebSockets and the Noise Protocol for end-to-end encryption.

## Key Features

- **End-to-End Encryption**: Noise Protocol with `Noise_XXpsk2_25519_AESGCM_SHA256`
- **Multi-Client Support**: Server handles unlimited concurrent clients using all CPU cores
- **Named Clients**: Each client provides a name for chat identification
- **Real-Time Chat**: Encrypted bidirectional communication with message broadcasting
- **High Performance**: Built with Tokio multi-threaded async runtime
- **Cross-Platform**: Works on Windows, macOS, and Linux

## Security Features

| Feature | Description |
|---------|-------------|
| **End-to-End Encryption** | All messages encrypted using AES-GCM |
| **Mutual Authentication** | Both client and server authenticate each other |
| **Pre-Shared Key** | Additional security layer with PSK authentication |
| **Perfect Forward Secrecy** | Session keys derived from ephemeral DH exchange |
| **Multi-Client Isolation** | Each client has independent encrypted session |

## Technology Stack

- **Language**: [Rust](https://www.rust-lang.org) 1.70+
- **Async Runtime**: [Tokio](https://tokio.rs) (multi-threaded)
- **WebSocket**: [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)
- **Encryption**: [Snow](https://github.com/mcginty/snow) (Noise Protocol)
- **Serialization**: [Serde](https://serde.rs)

## 📦 Installation

### Prerequisites

- **Rust** 1.70 or later
- **Cargo** package manager

### Quick Start

1. **Clone the repository**:
```bash
git clone https://github.com/HacksterAman/Secure-Web-Socket.git
cd Secure-Web-Socket
```

2. **Build the project**:
```bash
cargo build --release
```

3. **Run the server**:
```bash
cargo run --bin server
```

4. **Run the client** (in another terminal):
```bash
cargo run --bin client
```

## Usage

### Server

Start the server to accept multiple client connections:

```bash
cargo run --bin server
```

The server can send messages to clients:
- **Broadcast to all**: Just type your message
- **Send to specific client**: Use `@ClientName message`

Example output:
```
Server listening on: 127.0.0.1:8080
Using Noise protocol: Noise_XXpsk2_25519_AESGCM_SHA256
Commands: '@ClientName message' to send to specific client, or 'message' to broadcast
New connection from: 127.0.0.1:54321
WebSocket connection established
Starting Noise handshake...
Secure channel established
Alice joined the chat
Alice: Hello everyone!
Bob joined the chat
Alice: Hi Bob!
Bob: Hey Alice!
> Welcome to the chat!
Broadcast: Welcome to the chat!
> @Alice How are you?
To Alice: How are you?
Alice: I'm good, thanks!
```

### Client

Connect to the server and join the chat:

```bash
cargo run --bin client
```

The client will:
1. Connect to the server
2. Complete secure handshake
3. Be prompted to enter a name
4. Start chatting with other clients

Example session:
```
Connecting to server at: ws://127.0.0.1:8080
Connected to server
Starting Noise handshake...
Secure channel established
Server: Please enter your name:
> Alice
Server: Alice joined the chat
> Hello everyone!
Server: Bob joined the chat
Bob: Hey Alice!
> Hi Bob!
> quit
Disconnecting...
Disconnected
```

## Configuration

### Server Settings

Modify server address in `src/server.rs`:

```rust
const NOISE_PATTERN: &str = "Noise_XXpsk2_25519_AESGCM_SHA256";
const PSK: &[u8; 32] = b"my_super_secret_pre_shared_key!!";  // Change this!
```

### Client Settings

Modify server URL in `src/client.rs`:

```rust
let url = "ws://127.0.0.1:8080";
```

## Architecture

### System Overview

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Client A   │────▶│             │◀────│  Client B   │
│             │     │   Server    │     │             │
└─────────────┘     │(Multi-Core) │     └─────────────┘
                    │             │
┌─────────────┐     │  Broadcast  │     ┌─────────────┐
│  Client C   │────▶│   System    │◀────│  Client D   │
│             │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Security Flow

1. **Handshake**: Client and server perform Noise protocol XX handshake
2. **Authentication**: Both parties authenticate using ephemeral and static keys
3. **Name Exchange**: Server requests client name, client responds
4. **Chat**: All messages are encrypted, decrypted, and broadcasted to other clients
5. **Isolation**: Each client has independent encrypted session

## Security Analysis

### Threat Protection

| Threat | Protection |
|--------|------------|
| **Eavesdropping** | AES-GCM encryption |
| **MITM Attacks** | Mutual authentication |
| **Replay Attacks** | Noise protocol nonces |
| **Key Compromise** | Perfect Forward Secrecy |

### Cryptographic Properties

- **Confidentiality**: AES-256-GCM encryption
- **Authenticity**: AEAD authentication tags
- **Integrity**: Cryptographic message authentication
- **Forward Secrecy**: Ephemeral key exchange

## Development

### Project Structure

```
src/
├── server.rs          # Multi-client WebSocket server
├── client.rs          # WebSocket client
Cargo.toml            # Dependencies and metadata
README.md             # Documentation
LICENSE              # MIT license
```

### Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run server
cargo run --bin server

# Run client
cargo run --bin client
```

### Dependencies

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
tokio-tungstenite = "0.20"
futures-util = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
snow = "0.9"              # Noise protocol implementation
```

## Security Notes

**Important Security Considerations**

### Production Deployment

1. **Key Management**: Replace hardcoded PSK with proper key distribution
2. **Network Security**: Use TLS for transport layer security
3. **Authentication**: Implement robust user authentication
4. **Monitoring**: Add logging and intrusion detection
5. **Key Storage**: Secure storage for static keys

### Current Limitations

- Uses hardcoded pre-shared key (demo purposes only)
- No persistent storage (messages are not saved)
- Basic error handling (can be enhanced for production)

## Changelog

### v2.1.0 (Latest)
- Server can send messages to clients using client names
- Support for targeted messages (@ClientName) and broadcasts
- Interactive server command interface

### v2.0.0
- Multi-client support with broadcast messaging
- Multi-threaded server using all CPU cores
- Client name identification system
- Simplified codebase and removed emojis
- Cleaner architecture

### v1.0.0
- WebSocket communication
- Noise protocol integration
- End-to-end encryption
- JSON message format

## 🙏 Acknowledgments

- [Noise Protocol](https://noiseprotocol.org) for the cryptographic framework
- [Snow](https://github.com/mcginty/snow) for the Rust Noise implementation
- [Tokio](https://tokio.rs) for the async runtime
- [WebSocket RFC 6455](https://tools.ietf.org/html/rfc6455) for the communication protocol

## 📖 Further Reading

- [Noise Protocol Specification](https://noiseprotocol.org/noise.html)
- [WebSocket Security](https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers)
- [Rust Async Programming](https://rust-lang.github.io/async-book/)
- [Cryptographic Best Practices](https://github.com/veorq/cryptocoding)

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

<div align="center">

**Built with ❤️ and Rust**

[⭐ Star this repo](https://github.com/yourusername/secure-websocket-chat) if you find it useful!

</div> 
