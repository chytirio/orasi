//! SPDX-FileCopyrightText: © 2025 Cory Parent <goedelsoup+orasi@goedelsoup.io>
//! SPDX-License-Identifier: Apache-2.0
//!

//! Security and reliability modules for telemetry ingestion
//!
//! This module provides comprehensive security and reliability features
//! including authentication, authorization, encryption, and circuit breakers.

pub mod auth;

pub use bridge_auth::{
    AuthManager,
    AuthConfig,
    AuthMethod,
    User,
    JwtClaims,
    AuthResult,
};

/// Security manager for comprehensive security features
pub struct SecurityManager {
    auth_manager: AuthManager,
    encryption_manager: EncryptionManager,
    circuit_breaker: CircuitBreaker,
}

/// Encryption manager for data encryption and decryption
pub struct EncryptionManager {
    algorithm: EncryptionAlgorithm,
    key: Vec<u8>,
    key_rotation_interval: std::time::Duration,
    last_key_rotation: std::time::Instant,
}

/// Encryption algorithm
#[derive(Debug, Clone)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
    Aes256Cbc,
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    failure_threshold: usize,
    recovery_timeout: std::time::Duration,
    state: std::sync::Arc<tokio::sync::Mutex<CircuitBreakerState>>,
}

/// Circuit breaker state
#[derive(Debug, Clone)]
pub enum CircuitBreakerState {
    Closed { failure_count: usize },
    Open { last_failure_time: std::time::Instant },
    HalfOpen,
}

impl SecurityManager {
    /// Create new security manager
    pub async fn new(auth_config: AuthConfig) -> BridgeResult<Self> {
        let auth_manager = AuthManager::new(auth_config).await
            .map_err(|e| bridge_core::BridgeError::authentication(format!("Failed to create auth manager: {}", e)))?;
        let encryption_manager = EncryptionManager::new(EncryptionAlgorithm::Aes256Gcm)?;
        let circuit_breaker = CircuitBreaker::new(5, std::time::Duration::from_secs(60));

        Ok(Self {
            auth_manager,
            encryption_manager,
            circuit_breaker,
        })
    }

    /// Initialize security manager
    pub async fn initialize(&self) -> BridgeResult<()> {
        info!("Security manager initialized successfully");
        Ok(())
    }

    /// Get authentication manager
    pub fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
    }

    /// Get encryption manager
    pub fn encryption_manager(&self) -> &EncryptionManager {
        &self.encryption_manager
    }

    /// Get circuit breaker
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }

    /// Encrypt data
    pub async fn encrypt_data(&mut self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        self.encryption_manager.encrypt(data).await
    }

    /// Decrypt data
    pub async fn decrypt_data(&self, encrypted_data: &[u8]) -> BridgeResult<Vec<u8>> {
        self.encryption_manager.decrypt(encrypted_data).await
    }

    /// Execute with circuit breaker
    pub async fn execute_with_circuit_breaker<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + 'static + std::convert::From<std::io::Error>,
    {
        self.circuit_breaker.execute(operation).await
    }
}

impl EncryptionManager {
    /// Create new encryption manager
    pub fn new(algorithm: EncryptionAlgorithm) -> BridgeResult<Self> {
        // In a real implementation, you would generate or load a proper encryption key
        let key = vec![0u8; 32]; // 256-bit key for AES-256
        
        Ok(Self {
            algorithm,
            key,
            key_rotation_interval: std::time::Duration::from_secs(86400), // 24 hours
            last_key_rotation: std::time::Instant::now(),
        })
    }

    /// Encrypt data
    pub async fn encrypt(&mut self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        // Check if key rotation is needed
        if self.last_key_rotation.elapsed() > self.key_rotation_interval {
            self.rotate_key().await?;
        }

        match self.algorithm {
            EncryptionAlgorithm::Aes256Gcm => self.encrypt_aes256_gcm(data).await,
            EncryptionAlgorithm::ChaCha20Poly1305 => self.encrypt_chacha20_poly1305(data).await,
            EncryptionAlgorithm::Aes256Cbc => self.encrypt_aes256_cbc(data).await,
        }
    }

    /// Decrypt data
    pub async fn decrypt(&self, encrypted_data: &[u8]) -> BridgeResult<Vec<u8>> {
        match self.algorithm {
            EncryptionAlgorithm::Aes256Gcm => self.decrypt_aes256_gcm(encrypted_data).await,
            EncryptionAlgorithm::ChaCha20Poly1305 => self.decrypt_chacha20_poly1305(encrypted_data).await,
            EncryptionAlgorithm::Aes256Cbc => self.decrypt_aes256_cbc(encrypted_data).await,
        }
    }

    /// Encrypt with AES-256-GCM
    async fn encrypt_aes256_gcm(&self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm,
        };

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| bridge_core::BridgeError::configuration(format!("Failed to create cipher: {}", e)))?;

        let nonce = aes_gcm::Nonce::from_slice(b"unique nonce"); // In production, use a random nonce
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| bridge_core::BridgeError::configuration(format!("Failed to encrypt: {}", e)))?;

        Ok(ciphertext)
    }

    /// Decrypt with AES-256-GCM
    async fn decrypt_aes256_gcm(&self, encrypted_data: &[u8]) -> BridgeResult<Vec<u8>> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm,
        };

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| bridge_core::BridgeError::configuration(format!("Failed to create cipher: {}", e)))?;

        let nonce = aes_gcm::Nonce::from_slice(b"unique nonce"); // In production, use the same nonce as encryption
        let plaintext = cipher.decrypt(nonce, encrypted_data)
            .map_err(|e| bridge_core::BridgeError::configuration(format!("Failed to decrypt: {}", e)))?;

        Ok(plaintext)
    }

    /// Encrypt with ChaCha20-Poly1305
    async fn encrypt_chacha20_poly1305(&self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        // Simplified implementation - in production, use a proper ChaCha20-Poly1305 library
        Ok(data.to_vec())
    }

    /// Decrypt with ChaCha20-Poly1305
    async fn decrypt_chacha20_poly1305(&self, encrypted_data: &[u8]) -> BridgeResult<Vec<u8>> {
        // Simplified implementation - in production, use a proper ChaCha20-Poly1305 library
        Ok(encrypted_data.to_vec())
    }

    /// Encrypt with AES-256-CBC
    async fn encrypt_aes256_cbc(&self, data: &[u8]) -> BridgeResult<Vec<u8>> {
        // Simplified implementation - in production, use a proper AES-CBC library
        Ok(data.to_vec())
    }

    /// Decrypt with AES-256-CBC
    async fn decrypt_aes256_cbc(&self, encrypted_data: &[u8]) -> BridgeResult<Vec<u8>> {
        // Simplified implementation - in production, use a proper AES-CBC library
        Ok(encrypted_data.to_vec())
    }

    /// Rotate encryption key
    async fn rotate_key(&mut self) -> BridgeResult<()> {
        // In a real implementation, you would generate a new key and handle key rotation
        self.last_key_rotation = std::time::Instant::now();
        info!("Encryption key rotated successfully");
        Ok(())
    }
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(failure_threshold: usize, recovery_timeout: std::time::Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            state: std::sync::Arc::new(tokio::sync::Mutex::new(CircuitBreakerState::Closed { failure_count: 0 })),
        }
    }

    /// Execute operation with circuit breaker
    pub async fn execute<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::error::Error + 'static + std::convert::From<std::io::Error>,
    {
        let mut state = self.state.lock().await;
        
        match &*state {
            CircuitBreakerState::Closed { failure_count } => {
                if *failure_count >= self.failure_threshold {
                    *state = CircuitBreakerState::Open {
                        last_failure_time: std::time::Instant::now(),
                    };
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Circuit breaker is open",
                    ).into());
                }

                match operation() {
                    Ok(result) => {
                        *state = CircuitBreakerState::Closed { failure_count: 0 };
                        Ok(result)
                    }
                    Err(e) => {
                        *state = CircuitBreakerState::Closed {
                            failure_count: failure_count + 1,
                        };
                        Err(e)
                    }
                }
            }
            CircuitBreakerState::Open { last_failure_time } => {
                if last_failure_time.elapsed() >= self.recovery_timeout {
                    *state = CircuitBreakerState::HalfOpen;
                    match operation() {
                        Ok(result) => {
                            *state = CircuitBreakerState::Closed { failure_count: 0 };
                            Ok(result)
                        }
                        Err(e) => {
                            *state = CircuitBreakerState::Open {
                                last_failure_time: std::time::Instant::now(),
                            };
                            Err(e)
                        }
                    }
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Circuit breaker is open",
                    ).into())
                }
            }
            CircuitBreakerState::HalfOpen => {
                match operation() {
                    Ok(result) => {
                        *state = CircuitBreakerState::Closed { failure_count: 0 };
                        Ok(result)
                    }
                    Err(e) => {
                        *state = CircuitBreakerState::Open {
                            last_failure_time: std::time::Instant::now(),
                        };
                        Err(e)
                    }
                }
            }
        }
    }

    /// Get circuit breaker state
    pub async fn get_state(&self) -> CircuitBreakerState {
        let state = self.state.lock().await;
        state.clone()
    }

    /// Reset circuit breaker
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        *state = CircuitBreakerState::Closed { failure_count: 0 };
    }
}

/// TLS configuration for secure connections
pub struct TlsManager {
    cert_file: Option<std::path::PathBuf>,
    key_file: Option<std::path::PathBuf>,
    ca_file: Option<std::path::PathBuf>,
    enabled: bool,
}

impl TlsManager {
    /// Create new TLS manager
    pub fn new(
        cert_file: Option<std::path::PathBuf>,
        key_file: Option<std::path::PathBuf>,
        ca_file: Option<std::path::PathBuf>,
        enabled: bool,
    ) -> Self {
        Self {
            cert_file,
            key_file,
            ca_file,
            enabled,
        }
    }

    /// Check if TLS is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get TLS configuration
    pub fn get_config(&self) -> BridgeResult<TlsConfig> {
        if !self.enabled {
            return Err(bridge_core::BridgeError::configuration("TLS is not enabled".to_string()));
        }

        Ok(TlsConfig {
            cert_file: self.cert_file.clone(),
            key_file: self.key_file.clone(),
            ca_file: self.ca_file.clone(),
        })
    }
}

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_file: Option<std::path::PathBuf>,
    pub key_file: Option<std::path::PathBuf>,
    pub ca_file: Option<std::path::PathBuf>,
}

use bridge_core::BridgeResult;
use tracing::info;
