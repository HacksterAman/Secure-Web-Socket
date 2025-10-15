use serde::Deserialize;
use std::error::Error;
use std::fmt;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct QkdConfig {
    pub general: GeneralConfig,
    pub alice: EntityConfig,
    pub bob: EntityConfig,
    pub server: EntityConfig,
    pub kme: KmeConfig,
    pub noise: NoiseConfig,
    pub websocket: WebSocketConfig,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    pub kme_url: String,
    pub key_size: usize,
    pub timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EntityConfig {
    pub name: String,
    pub sae_id: String,
    pub kme_id: String,
    pub cert_file: String,
    pub key_file: String,
    pub pem_file: String,
    pub root_ca_file: String,
    pub kme_ca_file: String,
    pub kme_hostname: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct KmeConfig {
    pub base_url: String,
    pub status_endpoint: String,
    pub enc_keys_endpoint: String,
    pub dec_keys_endpoint: String,
}

#[derive(Debug, Deserialize)]
pub struct NoiseConfig {
    pub pattern: String,
    pub psk_position: u8,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketConfig {
    pub server_address: String,
}

#[derive(Debug)]
pub enum QkdError {
    ConfigError(String),
    CertificateError(String),
    KeyRetrievalError(String),
    NetworkError(String),
}

impl fmt::Display for QkdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QkdError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            QkdError::CertificateError(msg) => write!(f, "Certificate error: {}", msg),
            QkdError::KeyRetrievalError(msg) => write!(f, "Key retrieval error: {}", msg),
            QkdError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl Error for QkdError {}

impl QkdConfig {
    /// Load configuration from TOML file
    pub fn load(path: &str) -> Result<Self, QkdError> {
        let content = fs::read_to_string(path)
            .map_err(|e| QkdError::ConfigError(format!("Failed to read config file: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| QkdError::ConfigError(format!("Failed to parse config: {}", e)))
    }
}

/// QKD Client for key retrieval
pub struct QkdClient {
    config: EntityConfig,
    kme_url: String,
    key_size: usize,
}

impl QkdClient {
    pub fn new(entity_config: EntityConfig, kme_url: String, key_size: usize) -> Self {
        Self {
            config: entity_config,
            kme_url,
            key_size,
        }
    }

    /// Retrieve a QKD key from the KME using ETSI GS QKD 014 API
    pub async fn get_key(&self, target_sae_id: &str) -> Result<Vec<u8>, QkdError> {
        println!("\n[{} QKD] Retrieving key from KME", self.config.name);
        println!("[{} QKD] Source: {} (KME: {}, {})", 
            self.config.name, self.config.sae_id, self.config.kme_id, self.config.kme_hostname);
        println!("[{} QKD] Target: {}", self.config.name, target_sae_id);

        let key = self.retrieve_qkd_key_from_api(target_sae_id).await?;
        
        println!("[{} QKD] ✓ Key retrieved successfully ({} bytes)", self.config.name, key.len());
        println!("[{} QKD] Key (hex): {}\n", self.config.name, hex::encode(&key));

        Ok(key)
    }

    /// Retrieve QKD key from real KME API using direct HTTP request
    async fn retrieve_qkd_key_from_api(&self, target_sae_id: &str) -> Result<Vec<u8>, QkdError> {
        use reqwest;
        use serde::{Deserialize, Serialize};
        
        // Request body for ETSI GS QKD 014 API
        #[derive(Serialize)]
        struct KeyRequest {
            number: u32,
            size: u32,
        }
        
        // Response format from KME
        #[derive(Deserialize)]
        struct KeyResponse {
            keys: Vec<KeyData>,
        }
        
        #[derive(Deserialize)]
        struct KeyData {
            key_ID: String,
            key: String,  // Base64 encoded
        }
        
        // Load certificate and key for mTLS
        // The PEM file should already contain both cert and key
        let pem_data = std::fs::read(&self.config.pem_file)
            .map_err(|e| QkdError::CertificateError(format!("Failed to read PEM: {}", e)))?;
        
        let ca_pem = std::fs::read(&self.config.kme_ca_file)
            .map_err(|e| QkdError::CertificateError(format!("Failed to read CA: {}", e)))?;

        // Create mTLS identity from PEM (contains both cert and private key)
        let identity = reqwest::Identity::from_pem(&pem_data)
            .map_err(|e| QkdError::CertificateError(format!("Failed to create identity: {}", e)))?;
        
        // Create CA certificate
        let ca_cert = reqwest::Certificate::from_pem(&ca_pem)
            .map_err(|e| QkdError::CertificateError(format!("Failed to load CA cert: {}", e)))?;

        // Build HTTPS client with mTLS
        let client = reqwest::Client::builder()
            .identity(identity)
            .add_root_certificate(ca_cert)
            .build()
            .map_err(|e| QkdError::NetworkError(format!("Failed to build client: {}", e)))?;

        // Prepare request
        let url = format!(
            "https://{}/api/v1/keys/{}/enc_keys",
            self.config.kme_hostname,
            target_sae_id
        );
        
        let request_body = KeyRequest {
            number: 1,
            size: (self.key_size * 8) as u32,  // Convert bytes to bits
        };

        // Make POST request to KME
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| QkdError::NetworkError(format!("Request failed: {}", e)))?;

        // Check response status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(QkdError::KeyRetrievalError(format!(
                "API returned error {}: {}",
                status, body
            )));
        }

        // Parse response
        let key_response: KeyResponse = response
            .json()
            .await
            .map_err(|e| QkdError::KeyRetrievalError(format!("Failed to parse response: {}", e)))?;

        // Extract and decode the key
        if let Some(key_data) = key_response.keys.first() {
            let key_bytes = data_encoding::BASE64
                .decode(key_data.key.as_bytes())
                .map_err(|e| QkdError::KeyRetrievalError(format!("Failed to decode key: {}", e)))?;
            
            // Ensure correct key size
            if key_bytes.len() == self.key_size {
                Ok(key_bytes)
            } else {
                Err(QkdError::KeyRetrievalError(format!(
                    "Key size mismatch: expected {}, got {}",
                    self.key_size, key_bytes.len()
                )))
            }
        } else {
            Err(QkdError::KeyRetrievalError("No keys returned from KME".into()))
        }
    }


    /// Load certificates for mTLS authentication
    #[allow(dead_code)]
    pub fn load_certificates(&self) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), QkdError> {
        let cert = fs::read(&self.config.cert_file)
            .map_err(|e| QkdError::CertificateError(format!("Failed to read cert: {}", e)))?;
        
        let key = fs::read(&self.config.key_file)
            .map_err(|e| QkdError::CertificateError(format!("Failed to read key: {}", e)))?;
        
        let ca_cert = fs::read(&self.config.kme_ca_file)
            .map_err(|e| QkdError::CertificateError(format!("Failed to read CA cert: {}", e)))?;

        Ok((cert, key, ca_cert))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_qkd_key_consistency() {
        let alice_config = EntityConfig {
            name: "Alice".to_string(),
            sae_id: "sae-1".to_string(),
            kme_id: "kme-1".to_string(),
            cert_file: "sae-1.crt".to_string(),
            key_file: "sae-1.key".to_string(),
            pem_file: "sae-1.pem".to_string(),
            root_ca_file: "client-root-ca.crt".to_string(),
            kme_ca_file: "kme-ca.crt".to_string(),
            kme_hostname: "kme-1.example.com".to_string(),
            role: "initiator".to_string(),
        };

        let bob_config = EntityConfig {
            name: "Bob".to_string(),
            sae_id: "sae-2".to_string(),
            kme_id: "kme-2".to_string(),
            cert_file: "sae-2.crt".to_string(),
            key_file: "sae-2.key".to_string(),
            pem_file: "sae-2.pem".to_string(),
            root_ca_file: "client-root-ca.crt".to_string(),
            kme_ca_file: "kme-ca.crt".to_string(),
            kme_hostname: "kme-2.example.com".to_string(),
            role: "responder".to_string(),
        };

        let alice_client = QkdClient::new(alice_config.clone(), "https://kme.local".to_string(), 32);
        let bob_client = QkdClient::new(bob_config.clone(), "https://kme.local".to_string(), 32);

        let alice_key = alice_client.get_key("sae-2").await.unwrap();
        let bob_key = bob_client.get_key("sae-1").await.unwrap();

        // Both should derive the same key
        assert_eq!(alice_key, bob_key);
        assert_eq!(alice_key.len(), 32);
    }
}

