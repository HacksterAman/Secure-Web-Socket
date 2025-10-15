# 🚀 Run the QKD Demo - Complete Guide

## Quick Start (3 Commands)

```bash
# Terminal 1
cargo run --bin qkd_server

# Terminal 2 (new terminal)
cargo run --bin alice

# Terminal 3 (new terminal)
cargo run --bin bob
```

## What You'll See

### Complete Flow with Logging

#### Step 1: Start the Server

```bash
cargo run --bin qkd_server
```

**Expected Output:**
```
======================================================================
  QKD-ENABLED SECURE WEBSOCKET SERVER
  ETSI GS QKD 014 Standard Implementation
======================================================================

[Server] Loading configuration from qkd_config.toml
[Server] SAE ID: sae-3
[Server] KME ID: kme-3
[Server] KME Hostname: kme-3.acct-2761.etsi-qkd-api.qukaydee.com

[Server] Pre-retrieving QKD keys for known clients...
[Server] Retrieving key for Alice (sae-1)...

╔═══════════════════════════════════════════════════════════╗
║         Server        QKD KEY RETRIEVAL             ║
╠═══════════════════════════════════════════════════════════╣
║ Source SAE ID: sae-3                                      ║
║ Source KME ID: kme-3                                      ║
║ Target SAE ID: sae-1                                      ║
║ KME Hostname:  kme-3.acct-2761.etsi-qkd-api.qukaydee.com  ║
╚═══════════════════════════════════════════════════════════╝
ℹ [Server] Using simulated QKD (ETSI GS QKD 014 compliant)
→ [Server] Simulating QKD exchange...
  [Server] Key material ID: QKD_KEY_sae-1_sae-3_ETSI014
✓ [Server] Simulated QKD key generated
✓ [Server] QKD key retrieved (size: 32 bytes)
  [Server] Key (first 16 bytes): 7f0e1d2c3b4a59687776655443322110
  [Server] Key (full hex): 7f0e1d2c3b4a5968777665544332211000112233...

[Server] ✓ Alice key retrieved
[Server] Retrieving key for Bob (sae-2)...

╔═══════════════════════════════════════════════════════════╗
║         Server        QKD KEY RETRIEVAL             ║
╠═══════════════════════════════════════════════════════════╣
║ Source SAE ID: sae-3                                      ║
║ Source KME ID: kme-3                                      ║
║ Target SAE ID: sae-2                                      ║
║ KME Hostname:  kme-3.acct-2761.etsi-qkd-api.qukaydee.com  ║
╚═══════════════════════════════════════════════════════════╝
→ [Server] Simulating QKD exchange...
✓ [Server] QKD key retrieved (size: 32 bytes)

[Server] ✓ Bob key retrieved
[Server] All QKD keys successfully retrieved!

======================================================================
[Server] Listening on: 127.0.0.1:8080
[Server] Noise Protocol: Noise_XXpsk2_25519_AESGCM_SHA256
======================================================================

Server> 
```

Server is now ready and waiting for connections!

#### Step 2: Start Alice

```bash
cargo run --bin alice
```

**Expected Output:**
```
============================================================
  ALICE - QKD-Enabled Secure WebSocket Client
  ETSI GS QKD 014 Standard Implementation
============================================================

[Alice] Loading configuration from qkd_config.toml
[Alice] SAE ID: sae-1
[Alice] Role: initiator

[Alice] Initiating QKD key retrieval...
[Alice] Target: Server (sae-3)

╔═══════════════════════════════════════════════════════════╗
║         Alice         QKD KEY RETRIEVAL             ║
╠═══════════════════════════════════════════════════════════╣
║ Source SAE ID: sae-1                                      ║
║ Source KME ID: kme-1                                      ║
║ Target SAE ID: sae-3                                      ║
║ KME Hostname:  kme-1.acct-2761.etsi-qkd-api.qukaydee.com  ║
╚═══════════════════════════════════════════════════════════╝
→ [Alice] Simulating QKD exchange...
✓ [Alice] QKD key retrieved (size: 32 bytes)
  [Alice] Key (first 16 bytes): 7f0e1d2c3b4a59687776655443322110

[Alice] QKD key successfully retrieved and configured as PSK

[Alice] Connecting to server at: ws://127.0.0.1:8080
[Alice] WebSocket connection established
[Alice] Identified to server as Alice
[Alice] Starting Noise Protocol handshake with QKD-derived PSK...

╔═══════════════════════════════════════════════════════════╗
║         ALICE - NOISE PROTOCOL HANDSHAKE                 ║
╠═══════════════════════════════════════════════════════════╣
║ Pattern: Noise_XXpsk2_25519_AESGCM_SHA256               ║
║ PSK (first 16 bytes): 7f0e1d2c3b4a59687776655443322110  ║
╚═══════════════════════════════════════════════════════════╝

[Alice Handshake] Step 1: Sending initiator message...
[Alice Handshake] → Sending 48 bytes
[Alice Handshake] Step 2: Waiting for responder message...
[Alice Handshake] ← Received 96 bytes
[Alice Handshake] Step 3: Sending final message...
[Alice Handshake] → Sending 64 bytes
[Alice Handshake] ✓ Entering transport mode
[Alice Handshake] ✓ Handshake complete!

[Alice] ✓ Secure channel established with quantum-safe PSK

============================================================
  You are now chatting as Alice
  Type 'quit' to disconnect
============================================================

Alice> 
```

**Server will show:**
```
[Server] New connection from: 127.0.0.1:xxxxx
[Server] WebSocket connection established
[Server] Client identified as: Alice
[Server] Using QKD-derived PSK for Alice

╔═══════════════════════════════════════════════════════════╗
║        SERVER - NOISE PROTOCOL HANDSHAKE                 ║
╚═══════════════════════════════════════════════════════════╝
[Server Handshake] Step 1: Waiting for initiator message...
[Server Handshake] ← Received 48 bytes
[Server Handshake] Step 2: Sending responder message...
[Server Handshake] → Sending 96 bytes
[Server Handshake] Step 3: Waiting for final message...
[Server Handshake] ← Received 64 bytes
[Server Handshake] ✓ Handshake complete!

[Server] ✓ Secure channel established with Alice
[Server] Alice joined the chat (QKD-secured)
Server> 
```

#### Step 3: Start Bob

```bash
cargo run --bin bob
```

Bob's output will be similar to Alice's, but showing his own SAE ID (sae-2) and KME (kme-2).

#### Step 4: Send Messages

**Alice types:**
```
Alice> Hello everyone!
```

**Alice's console shows:**
```
[Alice ENCRYPT] ═══════════════════════════════════
[Alice ENCRYPT] Plaintext size: 47 bytes
[Alice ENCRYPT] Plaintext preview: {"sender":"Alice","content":"Hello everyone!"}
[Alice ENCRYPT] Ciphertext size: 63 bytes
[Alice ENCRYPT] Ciphertext (first 32 bytes): a3f5e7d9b2c4f6e8a1b3c5d7f9e0a2b4...
[Alice ENCRYPT] ✓ Encryption successful

Alice> 
```

**Server receives and decrypts:**
```
[Server DECRYPT] ═══════════════════════════════════
[Server DECRYPT] Ciphertext size: 63 bytes
[Server DECRYPT] Ciphertext (first 32 bytes): a3f5e7d9b2c4f6e8a1b3c5d7f9e0a2b4...
[Server DECRYPT] Plaintext size: 47 bytes
[Server DECRYPT] Plaintext: {"sender":"Alice","content":"Hello everyone!"}
[Server DECRYPT] ✓ Decryption successful

[Alice] Hello everyone!
```

**Bob receives (server encrypts for Bob):**
```
[Server ENCRYPT] ═══════════════════════════════════
[Server ENCRYPT] Plaintext size: 47 bytes
[Server ENCRYPT] Plaintext preview: {"sender":"Alice","content":"Hello everyone!"}
[Server ENCRYPT] Ciphertext size: 63 bytes
[Server ENCRYPT] ✓ Encryption successful

[Bob DECRYPT] ═══════════════════════════════════
[Bob DECRYPT] Ciphertext size: 63 bytes
[Bob DECRYPT] Plaintext: {"sender":"Alice","content":"Hello everyone!"}
[Bob DECRYPT] ✓ Decryption successful

Alice: Hello everyone!
Bob> 
```

**Bob responds:**
```
Bob> Hi Alice! Quantum-safe chat!
```

**All parties see the full encryption/decryption logs**

## Key Observations

### 1. Matching QKD Keys

Compare the logs:

**Alice:**
```
  [Alice] Key (first 16 bytes): 7f0e1d2c3b4a59687776655443322110
```

**Server (for Alice):**
```
  [Server] Key (first 16 bytes): 7f0e1d2c3b4a59687776655443322110
```

✅ **Keys match!** Both parties have the same quantum key.

### 2. Complete Encryption Flow

For every message you send, you'll see:
1. **Encryption** on sender side (plaintext → ciphertext)
2. **Decryption** on receiver side (ciphertext → plaintext)
3. **Hex dumps** of all data
4. **Size verification** (ciphertext = plaintext + 16 bytes)

### 3. Noise Handshake

The 3-way handshake is fully logged:
- Step 1: 48 bytes (initiator hello + ephemeral key)
- Step 2: 96 bytes (responder hello + ephemeral key + encrypted payload)
- Step 3: 64 bytes (initiator confirmation + encrypted payload)

## Commands Summary

### Start Everything
```bash
# Terminal 1
cargo run --bin qkd_server

# Terminal 2
cargo run --bin alice

# Terminal 3
cargo run --bin bob
```

### Save Logs to Files
```bash
# Terminal 1
cargo run --bin qkd_server 2>&1 | tee logs/server.log

# Terminal 2
cargo run --bin alice 2>&1 | tee logs/alice.log

# Terminal 3
cargo run --bin bob 2>&1 | tee logs/bob.log
```

### Server Commands
```bash
# Broadcast to all
Server> Hello everyone!

# Send to Alice only
Server> @Alice Private message for you

# Send to Bob only
Server> @Bob How are you Bob?
```

### Exit
```
Alice> quit
Bob> quit
Server> Ctrl+C
```

## What the Logs Tell You

| Log Entry | Meaning |
|-----------|---------|
| `╔═══╗` | Major operation starting |
| `→` | Sending data |
| `←` | Receiving data |
| `✓` | Success |
| `[ENCRYPT]` | Encryption operation |
| `[DECRYPT]` | Decryption operation |
| `QKD KEY RETRIEVAL` | Getting key from KME |
| `NOISE PROTOCOL HANDSHAKE` | Secure channel establishment |

## Educational Value

This demo shows you:

1. **QKD Integration**: How quantum keys are retrieved and used
2. **Noise Protocol**: Complete handshake process with PSK
3. **AEAD Encryption**: Authenticated encryption with GCM
4. **Key Matching**: Both parties derive identical keys
5. **E2E Security**: Message encrypted at source, decrypted at destination
6. **Hex Analysis**: See actual ciphertext and verify encryption

## Next Steps

- Check `LOGGING_GUIDE.md` for detailed log explanations
- Check `QKD_DEMO_README.md` for architecture details
- Compare hex dumps to verify encryption
- Analyze key derivation process
- Study the 3-way Noise handshake

Enjoy exploring! 🚀🔐

