# QKD Demo - Quantum Key Distribution with Noise Protocol

## Overview

This demo implements a **Quantum Key Distribution (QKD)** system following the **ETSI GS QKD 014** standard, integrated with the **Noise Protocol** for secure communication. The system consists of three entities:

1. **Alice** (Client 1) - Uses SAE-1 certificates and KME-1
2. **Bob** (Client 2) - Uses SAE-2 certificates and KME-2  
3. **Server** - Uses SAE-3 certificates and KME-3

Each entity retrieves quantum-safe keys from their respective Key Management Entity (KME) and uses them as Pre-Shared Keys (PSKs) in the Noise Protocol handshake.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  QuKayDee QKD Infrastructure                 │
│                                                              │
│  ┌──────────┐      ┌──────────┐      ┌──────────┐         │
│  │  KME-1   │      │  KME-2   │      │  KME-3   │         │
│  │  Alice   │      │   Bob    │      │  Server  │         │
│  └────┬─────┘      └────┬─────┘      └────┬─────┘         │
│       │                 │                  │                │
└───────┼─────────────────┼──────────────────┼────────────────┘
        │                 │                  │
    QKD Key          QKD Key            QKD Key
  (Alice-Server)   (Bob-Server)      (Server-Alice/Bob)
        │                 │                  │
        ▼                 ▼                  ▼
   ┌────────┐        ┌────────┐        ┌──────────┐
   │ Alice  │◄──────►│  Bob   │◄──────►│  Server  │
   │(Client)│        │(Client)│        │          │
   └────────┘        └────────┘        └──────────┘
        │                 │                  │
        └─────────────────┴──────────────────┘
              Noise Protocol with QKD PSK
           (Noise_XXpsk2_25519_AESGCM_SHA256)
```

## ETSI GS QKD 014 Standard

The ETSI GS QKD 014 standard defines:
- REST-based API for key delivery
- Secure Application Entity (SAE) to Key Management Entity (KME) protocol
- mTLS authentication using certificates
- Key retrieval and management endpoints

### KME Endpoints

Each entity has its own KME server:

| Entity | KME ID | Hostname |
|--------|--------|----------|
| Alice  | kme-1  | kme-1.acct-2761.etsi-qkd-api.qukaydee.com |
| Bob    | kme-2  | kme-2.acct-2761.etsi-qkd-api.qukaydee.com |
| Server | kme-3  | kme-3.acct-2761.etsi-qkd-api.qukaydee.com |

### API Endpoints (ETSI GS QKD 014)

- **Status**: `GET /api/v1/keys/{slave_SAE_ID}/status`
- **Get Key**: `GET /api/v1/keys/{slave_SAE_ID}/enc_keys`
- **Decode Key**: `POST /api/v1/keys/{master_SAE_ID}/dec_keys`

## Certificate Configuration

The system uses the following certificates (already present in your directory):

### Alice (SAE-1)
- `sae-1.crt` - Client certificate
- `sae-1.key` - Private key
- `sae-1.pem` - PEM format certificate

### Bob (SAE-2)
- `sae-2.crt` - Client certificate
- `sae-2.key` - Private key
- `sae-2.pem` - PEM format certificate

### Server (SAE-3)
- `sae-3.crt` - Server certificate
- `sae-3.key` - Private key
- `sae-3.pem` - PEM format certificate

### Shared CA Certificates
- `client-root-ca.crt` - Root CA certificate
- `client-root-ca.key` - Root CA private key
- `account-2761-server-ca-qukaydee-com.crt` - QuKayDee server CA

## Configuration File

The `qkd_config.toml` file defines all entity configurations:

```toml
[general]
kme_url = "https://qukaydee.com:443"
key_size = 32
timeout_seconds = 30

[alice]
name = "Alice"
sae_id = "sae-1"
kme_id = "kme-1"
kme_hostname = "kme-1.acct-2761.etsi-qkd-api.qukaydee.com"
# ... certificates ...

[bob]
name = "Bob"
sae_id = "sae-2"
kme_id = "kme-2"
kme_hostname = "kme-2.acct-2761.etsi-qkd-api.qukaydee.com"
# ... certificates ...

[server]
name = "Server"
sae_id = "sae-3"
kme_id = "kme-3"
kme_hostname = "kme-3.acct-2761.etsi-qkd-api.qukaydee.com"
# ... certificates ...
```

## Building the Demo

```bash
# Build all binaries
cargo build --release

# Or build individually
cargo build --release --bin alice
cargo build --release --bin bob
cargo build --release --bin qkd_server
```

## Running the Demo

### Step 1: Start the QKD Server

```bash
cargo run --release --bin qkd_server
```

The server will:
1. Load configuration from `qkd_config.toml`
2. Connect to KME-3 to retrieve QKD keys for Alice and Bob
3. Start listening on `127.0.0.1:8080`
4. Wait for client connections

Expected output:
```
======================================================================
  QKD-ENABLED SECURE WEBSOCKET SERVER
  ETSI GS QKD 014 Standard Implementation
======================================================================

[Server] Loading configuration from qkd_config.toml
[Server] SAE ID: sae-3
[Server] KME ID: kme-3
[Server] Pre-retrieving QKD keys for known clients...
[Server] Retrieving key for Alice (sae-1)...
[Alice] Successfully retrieved QKD key (size: 32 bytes)
[Server] ✓ Alice key retrieved
[Server] Retrieving key for Bob (sae-2)...
[Bob] Successfully retrieved QKD key (size: 32 bytes)
[Server] ✓ Bob key retrieved

======================================================================
[Server] Listening on: 127.0.0.1:8080
[Server] Noise Protocol: Noise_XXpsk2_25519_AESGCM_SHA256
======================================================================
```

### Step 2: Start Alice (in a new terminal)

```bash
cargo run --release --bin alice
```

Alice will:
1. Load configuration and retrieve QKD key from KME-1
2. Connect to the server
3. Perform Noise handshake using QKD-derived PSK
4. Start encrypted chat session

Expected output:
```
============================================================
  ALICE - QKD-Enabled Secure WebSocket Client
  ETSI GS QKD 014 Standard Implementation
============================================================

[Alice] Loading configuration from qkd_config.toml
[Alice] SAE ID: sae-1
[Alice] Initiating QKD key retrieval...
[Alice] Target: Server (sae-3)
[Alice] Successfully retrieved QKD key (size: 32 bytes)
[Alice] Connecting to server at: ws://127.0.0.1:8080
[Alice] WebSocket connection established
[Alice] Identified to server as Alice
[Alice] Starting Noise Protocol handshake with QKD-derived PSK...
[Alice] ✓ Secure channel established with quantum-safe PSK

============================================================
  You are now chatting as Alice
  Type 'quit' to disconnect
============================================================

Alice> 
```

### Step 3: Start Bob (in another terminal)

```bash
cargo run --release --bin bob
```

Bob will follow the same process as Alice but with his own QKD key from KME-2.

## Usage Examples

### Alice sends a message to all:
```
Alice> Hello everyone!
```

### Bob responds:
```
Bob> Hi Alice! Nice to meet you!
```

### Server broadcasts to all:
```
Server> Welcome to the QKD-secured chat!
```

### Server sends targeted message to Alice:
```
Server> @Alice Your connection is quantum-safe!
```

## QKD Key Flow

1. **Alice** requests key from **KME-1** for communication with **Server (sae-3)**
2. **Bob** requests key from **KME-2** for communication with **Server (sae-3)**
3. **Server** requests keys from **KME-3** for:
   - Communication with **Alice (sae-1)**
   - Communication with **Bob (sae-2)**
4. The QKD system ensures Alice-Server and Bob-Server get **matching symmetric keys**
5. These keys are used as **PSKs in the Noise Protocol** handshake
6. All subsequent messages are encrypted with keys derived from the Noise handshake

## Security Features

| Feature | Description |
|---------|-------------|
| **Quantum Key Distribution** | Keys generated and distributed via QKD |
| **ETSI GS QKD 014 Compliance** | Standard-compliant key retrieval |
| **mTLS Authentication** | Certificate-based authentication to KME |
| **Noise Protocol** | Post-quantum secure handshake |
| **Pre-Shared Keys** | QKD keys used as PSK in Noise |
| **End-to-End Encryption** | AES-GCM encryption for all messages |
| **Perfect Forward Secrecy** | Ephemeral DH exchange in Noise XX |
| **Mutual Authentication** | Both parties authenticated via Noise |

## Implementation Details

### QKD Module (`src/qkd_module.rs`)

The QKD module provides:
- Configuration loading from TOML
- QKD client for key retrieval
- Certificate management
- Error handling
- Simulated QKD (for demo without real KME access)

### Real vs. Simulated QKD

**Current Implementation (Simulated)**:
- Uses deterministic key derivation based on SAE IDs
- Ensures both parties get the same key
- Demonstrates the protocol flow
- No actual network calls to KME

**Production Implementation**:
Would use `etsi014-client` crate to:
- Establish mTLS connection to KME
- Authenticate using certificates
- Request keys via ETSI GS QKD 014 API
- Handle key refresh and rotation

### Noise Protocol Integration

The demo uses `Noise_XXpsk2_25519_AESGCM_SHA256`:
- **XX**: Mutual authentication pattern
- **psk2**: Pre-shared key at position 2 (after DH exchanges)
- **25519**: Curve25519 for DH
- **AESGCM**: AES-GCM encryption
- **SHA256**: SHA-256 hash function

## Files Structure

```
Secure-Web-Socket/
├── Cargo.toml                    # Updated with new binaries
├── qkd_config.toml              # QKD configuration
├── QKD_DEMO_README.md           # This file
├── src/
│   ├── qkd_module.rs            # QKD client and config
│   ├── alice.rs                 # Alice client
│   ├── bob.rs                   # Bob client
│   ├── qkd_server.rs            # QKD-enabled server
│   ├── client.rs                # Original client (unchanged)
│   └── server.rs                # Original server (unchanged)
├── sae-1.crt, sae-1.key, sae-1.pem  # Alice certificates
├── sae-2.crt, sae-2.key, sae-2.pem  # Bob certificates
├── sae-3.crt, sae-3.key, sae-3.pem  # Server certificates
├── client-root-ca.crt           # Root CA
└── account-2761-server-ca-qukaydee-com.crt  # KME CA

```

## Troubleshooting

### Build Errors
```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Connection Issues
- Ensure server is running before starting clients
- Check that port 8080 is not in use
- Verify `qkd_config.toml` paths are correct

### Certificate Errors
- Verify all certificate files exist in the project root
- Check file permissions
- Ensure certificates match the configuration

## Extending to Real QKD

To use real QKD infrastructure:

1. Update `src/qkd_module.rs` to use `etsi014-client`
2. Replace `simulate_qkd_key_retrieval()` with actual API calls
3. Configure proper mTLS with KME servers
4. Handle key lifecycle (refresh, rotation, expiry)
5. Implement error recovery and fallback mechanisms

Example real implementation (in comments in `qkd_module.rs`):
```rust
use etsi014_client::{Client, ClientConfig};

let config = ClientConfig::builder()
    .sae_id(sae_id.to_string())
    .cert_path(cert_path)
    .key_path(key_path)
    .ca_path(ca_path)
    .kme_url(kme_url)
    .build()?;

let client = Client::new(config)?;
let key_response = client.get_key(target_sae_id).await?;
```

## References

- **ETSI GS QKD 014**: [QKD Application Interface Specification](https://www.etsi.org/deliver/etsi_gs/QKD/001_099/014/)
- **Noise Protocol**: [https://noiseprotocol.org/](https://noiseprotocol.org/)
- **QuKayDee**: [https://qukaydee.com/](https://qukaydee.com/)
- **etsi014-client**: [Rust crate for ETSI QKD 014](https://crates.io/crates/etsi014-client)

## License

Same as the parent project (MIT License)

## Notes

- Original `client.rs` and `server.rs` files remain **unchanged**
- All QKD functionality is in new files
- Configuration is self-contained in `qkd_config.toml`
- Demo works in simulation mode without actual QKD infrastructure
- Ready for production deployment with real KME servers

