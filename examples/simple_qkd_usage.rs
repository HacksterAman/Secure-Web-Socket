// Example: Simple QKD Key Retrieval
// 
// This demonstrates how to use the QKD library to get keys for PSK

use secure_websocket::get_key_for_user;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("QKD Library - Simple Usage Example\n");
    
    // Example 1: Get key for Alice to communicate with Server
    println!("Getting QKD key for Alice -> Server...");
    let alice_key = get_key_for_user("Alice", "Server").await?;
    println!("✓ Alice key retrieved: {} bytes", alice_key.len());
    println!("  Key (hex): {}\n", hex::encode(&alice_key));
    
    // This key can now be used as PSK:
    // const PSK: &[u8; 32] = &alice_key;
    
    // Example 2: Get key for Bob to communicate with Server
    println!("Getting QKD key for Bob -> Server...");
    let bob_key = get_key_for_user("Bob", "Server").await?;
    println!("✓ Bob key retrieved: {} bytes", bob_key.len());
    println!("  Key (hex): {}\n", hex::encode(&bob_key));
    
    // Example 3: Get key for Server to communicate with Alice
    println!("Getting QKD key for Server -> Alice...");
    let server_alice_key = get_key_for_user("Server", "Alice").await?;
    println!("✓ Server key (for Alice) retrieved: {} bytes", server_alice_key.len());
    println!("  Key (hex): {}\n", hex::encode(&server_alice_key));
    
    // Verify: Alice's key and Server's key for Alice should match
    if alice_key == server_alice_key {
        println!("✅ SUCCESS: Alice and Server have matching QKD keys!");
    } else {
        println!("❌ ERROR: Keys don't match!");
    }
    
    Ok(())
}

