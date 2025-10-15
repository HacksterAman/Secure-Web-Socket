# QKD Library Documentation

A simple Rust library for retrieving Quantum Key Distribution (QKD) keys via the ETSI GS QKD 014 API.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
secure-websocket = { path = "." }
```

## Quick Start

### 1. Async Usage (Recommended)

```rust
use secure_websocket::get_key_for_user;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get QKD key for Alice to communicate with Server
    let psk: [u8; 32] = get_key_for_user("Alice", "Server").await?;
    
    println!("Key retrieved: {} bytes", psk.len());
    println!("Key (hex): {}", hex::encode(&psk));
    
    Ok(())
}
```

### 2. Server Usage (Multiple Keys)

```rust
use secure_websocket::init_server_keys;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get keys for both Alice and Bob at once
    let (alice_key, bob_key) = init_server_keys().await?;
    
    println!("Alice key: {}", hex::encode(&alice_key));
    println!("Bob key: {}", hex::encode(&bob_key));
    
    Ok(())
}
```

### 3. Synchronous Usage

```rust
use secure_websocket::get_key_for_user_sync;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Synchronous wrapper (creates its own runtime)
    let psk = get_key_for_user_sync("Bob", "Server")?;
    
    println!("Key retrieved: {} bytes", psk.len());
    
    Ok(())
}
```

## API Reference

### `get_key_for_user(username, target_username)`

Retrieves a QKD key for communication between two users.

**Parameters:**
- `username: &str` - Source user ("Alice", "Bob", or "Server")
- `target_username: &str` - Target user to communicate with

**Returns:**
- `Result<[u8; 32], QkdError>` - 32-byte quantum key

**Example:**
```rust
let key = get_key_for_user("Alice", "Server").await?;
```

### `init_server_keys()`

Convenience function for servers to get keys for all known clients.

**Returns:**
- `Result<([u8; 32], [u8; 32]), QkdError>` - (alice_key, bob_key)

**Example:**
```rust
let (alice_key, bob_key) = init_server_keys().await?;
```

### `get_key_for_user_sync(username, target_username)`

Synchronous wrapper around `get_key_for_user`.

**Parameters:**
- Same as `get_key_for_user`

**Returns:**
- `Result<[u8; 32], QkdError>` - 32-byte quantum key

## Supported User Combinations

The library supports the following communication pairs:

| Source | Target | Description |
|--------|--------|-------------|
| Alice  | Server | Alice connecting to Server |
| Alice  | Bob    | Alice to Bob communication |
| Bob    | Server | Bob connecting to Server |
| Bob    | Alice  | Bob to Alice communication |
| Server | Alice  | Server communicating with Alice |
| Server | Bob    | Server communicating with Bob |

## Configuration

The library reads from `qkd_config.toml` in the current directory. Make sure this file exists with proper configuration:

```toml
key_size = 32
timeout_seconds = 30

[alice]
name = "Alice"
sae_id = "sae-1"
kme_id = "kme-1"
cert_file = "sae-1.crt"
key_file = "sae-1.key"
pem_file = "sae-1.pem"
# ... etc

[bob]
# ... configuration for Bob

[server]
# ... configuration for Server

[kme]
base_url = "https://qukaydee.com:443"
```

## Error Handling

The library returns `QkdError` for all errors:

```rust
use secure_websocket::{get_key_for_user, QkdError};

match get_key_for_user("Alice", "Server").await {
    Ok(key) => println!("Success: {}", hex::encode(&key)),
    Err(QkdError::ConfigError(e)) => eprintln!("Config error: {}", e),
    Err(QkdError::NetworkError(e)) => eprintln!("Network error: {}", e),
    Err(QkdError::CertificateError(e)) => eprintln!("Certificate error: {}", e),
    Err(QkdError::KeyRetrievalError(e)) => eprintln!("Key retrieval error: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Using as PSK in Noise Protocol

```rust
use secure_websocket::get_key_for_user;
use snow::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get QKD key
    let psk = get_key_for_user("Alice", "Server").await?;
    
    // Use in Noise Protocol
    let builder: Builder<'_> = Builder::new("Noise_XXpsk2_25519_AESGCM_SHA256".parse()?)
        .psk(2, &psk)?;
    
    let mut noise = builder.build_initiator()?;
    
    // ... rest of Noise handshake
    
    Ok(())
}
```

## Requirements

- Valid `qkd_config.toml` configuration file
- Valid certificates and keys for mTLS authentication
- Network access to the KME API
- Tokio async runtime

## Testing

Run the example to test your setup:

```bash
cargo run --release --example simple_qkd_usage
```

This will:
1. Retrieve a key for Alice → Server
2. Retrieve a key for Bob → Server
3. Retrieve a key for Server → Alice
4. Verify that Alice and Server have matching keys

## Troubleshooting

### "Failed to build client: builder error"
- Check that certificate files exist and paths are correct in `qkd_config.toml`
- Ensure certificates are in PEM format
- Verify that the certificate files are readable

### "Failed to read cert/key/CA"
- Verify file paths in `qkd_config.toml` are correct
- Make sure you're running from the directory containing the certificates
- Check file permissions

### "API returned error 400"
- Verify your SAE IDs and KME IDs are correct
- Check that the KME hostname is correct
- Ensure your certificates are valid and not expired

### "Network error: Request failed"
- Check network connectivity to the KME server
- Verify the KME hostname is correct
- Ensure the KME is reachable on port 443

## License

Same as the parent project.

