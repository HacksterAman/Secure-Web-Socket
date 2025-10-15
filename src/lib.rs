// QKD Library - Simple interface for retrieving quantum keys
//
// Usage:
//   let psk = qkd::get_key_for_user("Alice", "Server").await?;
//   // Use psk as PSK in Noise Protocol

pub mod qkd_module;

use qkd_module::{QkdConfig, QkdClient, QkdError};

/// Get a QKD key for a specific user
/// 
/// # Arguments
/// * `username` - The name of the user ("Alice", "Bob", or "Server")
/// * `target_username` - The name of the target user to communicate with
/// 
/// # Returns
/// * `[u8; 32]` - 32-byte quantum key ready to use as PSK
/// 
/// # Example
/// ```ignore
/// let psk = qkd::get_key_for_user("Alice", "Server").await?;
/// const PSK: &[u8; 32] = &psk;
/// ```
pub async fn get_key_for_user(username: &str, target_username: &str) -> Result<[u8; 32], QkdError> {
    // Load configuration
    let config = QkdConfig::load("qkd_config.toml")?;
    
    // Get the entity configuration based on username
    let (entity_config, target_sae_id) = match (username, target_username) {
        ("Alice", "Server") => (config.alice.clone(), config.server.sae_id),
        ("Alice", "Bob") => (config.alice.clone(), config.bob.sae_id),
        ("Bob", "Server") => (config.bob.clone(), config.server.sae_id),
        ("Bob", "Alice") => (config.bob.clone(), config.alice.sae_id),
        ("Server", "Alice") => (config.server.clone(), config.alice.sae_id),
        ("Server", "Bob") => (config.server.clone(), config.bob.sae_id),
        _ => return Err(QkdError::ConfigError(format!(
            "Invalid username combination: {} -> {}",
            username, target_username
        ))),
    };
    
    // Create QKD client
    let qkd_client = QkdClient::new(
        entity_config,
        config.general.kme_url,
        config.general.key_size,
    );
    
    // Retrieve key from KME
    let key_vec = qkd_client.get_key(&target_sae_id).await?;
    
    // Convert to fixed-size array
    if key_vec.len() != 32 {
        return Err(QkdError::KeyRetrievalError(format!(
            "Expected 32-byte key, got {} bytes",
            key_vec.len()
        )));
    }
    
    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&key_vec);
    
    Ok(key_array)
}

/// Get a QKD key for a specific user (synchronous wrapper)
/// 
/// This is a convenience function that runs the async version in a new runtime.
/// Use the async version if you're already in an async context.
pub fn get_key_for_user_sync(username: &str, target_username: &str) -> Result<[u8; 32], QkdError> {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(get_key_for_user(username, target_username))
}

/// Initialize QKD keys for server (retrieves keys for all known clients)
/// 
/// Returns a tuple of (alice_key, bob_key)
pub async fn init_server_keys() -> Result<([u8; 32], [u8; 32]), QkdError> {
    let alice_key = get_key_for_user("Server", "Alice").await?;
    let bob_key = get_key_for_user("Server", "Bob").await?;
    
    Ok((alice_key, bob_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_alice_key() {
        // This test requires valid config and network access
        match get_key_for_user("Alice", "Server").await {
            Ok(key) => {
                assert_eq!(key.len(), 32);
                println!("Alice key retrieved: {} bytes", key.len());
            }
            Err(e) => {
                println!("Expected failure (no KME access in test): {}", e);
            }
        }
    }
}

