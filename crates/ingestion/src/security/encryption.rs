use crate::BridgeResult;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::Rng;

/// Supported encryption algorithms
#[derive(Debug, Clone, Copy)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
}

/// Encryption manager for handling data encryption and decryption
pub struct EncryptionManager {
    algorithm: EncryptionAlgorithm,
    key: Vec<u8>,
}

impl EncryptionManager {
    /// Create new encryption manager
    pub fn new(algorithm: EncryptionAlgorithm) -> BridgeResult<Self> {
        let key = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let mut key = [0u8; 32];
                rand::thread_rng().fill(&mut key);
                key.to_vec()
            }
        };

        Ok(Self { algorithm, key })
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        match self.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|e| {
                    bridge_core::BridgeError::internal(format!("Failed to create cipher: {}", e))
                })?;
                let mut nonce_bytes = [0u8; 12];
                OsRng.fill(&mut nonce_bytes);
                let nonce = Nonce::from_slice(&nonce_bytes);

                let ciphertext = cipher.encrypt(&nonce, data).map_err(|e| {
                    bridge_core::BridgeError::internal(format!("Encryption failed: {}", e))
                })?;

                // Combine nonce and ciphertext
                let mut result = nonce.to_vec();
                result.extend(ciphertext);
                Ok(result)
            }
        }
    }

    /// Decrypt data
    pub fn decrypt(&self, encrypted_data: &[u8]) -> BridgeResult<Vec<u8>> {
        match self.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                if encrypted_data.len() < 12 {
                    return Err(bridge_core::BridgeError::internal(
                        "Invalid encrypted data length".to_string(),
                    ));
                }

                let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|e| {
                    bridge_core::BridgeError::internal(format!("Failed to create cipher: {}", e))
                })?;
                let nonce = Nonce::from_slice(&encrypted_data[..12]);
                let ciphertext = &encrypted_data[12..];

                let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
                    bridge_core::BridgeError::internal(format!("Decryption failed: {}", e))
                })?;

                Ok(plaintext)
            }
        }
    }

    /// Get the encryption algorithm
    pub fn algorithm(&self) -> EncryptionAlgorithm {
        self.algorithm
    }

    /// Check if encryption is enabled
    pub fn is_enabled(&self) -> bool {
        !self.key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let manager = EncryptionManager::new(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let test_data = b"Hello, World!";

        let encrypted = manager.encrypt(test_data).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();

        assert_eq!(test_data, decrypted.as_slice());
    }

    #[test]
    fn test_encryption_manager_creation() {
        let manager = EncryptionManager::new(EncryptionAlgorithm::Aes256Gcm).unwrap();
        assert!(manager.is_enabled());
        assert!(matches!(
            manager.algorithm(),
            EncryptionAlgorithm::Aes256Gcm
        ));
    }
}
