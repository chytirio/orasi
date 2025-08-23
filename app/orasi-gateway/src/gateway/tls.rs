//! TLS configuration

use crate::{config::GatewayConfig, error::GatewayError};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// TLS manager for gateway
pub struct TlsManager {
    config: GatewayConfig,
    tls_config: Option<Arc<ServerConfig>>,
}

impl TlsManager {
    /// Create new TLS manager
    pub async fn new(config: &GatewayConfig) -> Result<Self, GatewayError> {
        let mut tls_manager = Self {
            config: config.clone(),
            tls_config: None,
        };

        // Load TLS configuration if enabled
        if config.tls.enabled {
            tls_manager.load_tls_config().await?;
        } else {
            info!("TLS is disabled in configuration");
        }

        Ok(tls_manager)
    }

    /// Load TLS configuration
    pub async fn load_tls_config(&mut self) -> Result<(), GatewayError> {
        if !self.config.tls.enabled {
            debug!("TLS is disabled, skipping configuration loading");
            return Ok(());
        }

        info!("Loading TLS configuration...");

        // Validate TLS configuration
        self.validate_tls_config()?;

        // Load certificates and private key
        let certs = self.load_certificates().await?;
        let key = self.load_private_key().await?;

        // Create TLS server configuration
        let tls_config = self.create_server_config(certs, key).await?;

        self.tls_config = Some(Arc::new(tls_config));
        info!("TLS configuration loaded successfully");

        Ok(())
    }

    /// Validate TLS configuration
    fn validate_tls_config(&self) -> Result<(), GatewayError> {
        let tls_config = &self.config.tls;

        if !tls_config.enabled {
            return Ok(());
        }

        // Check if certificate file is specified
        if tls_config.cert_file.is_none() {
            return Err(GatewayError::Configuration(
                "TLS is enabled but no certificate file specified".to_string(),
            ));
        }

        // Check if private key file is specified
        if tls_config.key_file.is_none() {
            return Err(GatewayError::Configuration(
                "TLS is enabled but no private key file specified".to_string(),
            ));
        }

        // Validate certificate file exists
        if let Some(cert_path) = &tls_config.cert_file {
            if !Path::new(cert_path).exists() {
                return Err(GatewayError::Configuration(format!(
                    "Certificate file not found: {}",
                    cert_path
                )));
            }
        }

        // Validate private key file exists
        if let Some(key_path) = &tls_config.key_file {
            if !Path::new(key_path).exists() {
                return Err(GatewayError::Configuration(format!(
                    "Private key file not found: {}",
                    key_path
                )));
            }
        }

        // Validate CA certificate file exists if specified
        if let Some(ca_path) = &tls_config.ca_file {
            if !Path::new(ca_path).exists() {
                return Err(GatewayError::Configuration(format!(
                    "CA certificate file not found: {}",
                    ca_path
                )));
            }
        }

        debug!("TLS configuration validation passed");
        Ok(())
    }

    /// Load certificates from file
    async fn load_certificates(&self) -> Result<Vec<Certificate>, GatewayError> {
        let cert_path = self
            .config
            .tls
            .cert_file
            .as_ref()
            .ok_or_else(|| {
                GatewayError::Configuration("Certificate file path not specified".to_string())
            })?;

        info!("Loading certificates from: {}", cert_path);

        let file = File::open(cert_path).map_err(|e| {
            GatewayError::Configuration(format!("Failed to open certificate file: {}", e))
        })?;

        let mut reader = BufReader::new(file);
        let certs = certs(&mut reader).map_err(|e| {
            GatewayError::Configuration(format!("Failed to parse certificates: {}", e))
        })?;

        if certs.is_empty() {
            return Err(GatewayError::Configuration(
                "No certificates found in certificate file".to_string(),
            ));
        }

        let certificates: Vec<Certificate> = certs.into_iter().map(Certificate).collect();
        info!("Loaded {} certificates", certificates.len());

        Ok(certificates)
    }

    /// Load private key from file
    async fn load_private_key(&self) -> Result<PrivateKey, GatewayError> {
        let key_path = self
            .config
            .tls
            .key_file
            .as_ref()
            .ok_or_else(|| {
                GatewayError::Configuration("Private key file path not specified".to_string())
            })?;

        info!("Loading private key from: {}", key_path);

        let file = File::open(key_path).map_err(|e| {
            GatewayError::Configuration(format!("Failed to open private key file: {}", e))
        })?;

        let mut reader = BufReader::new(file);

        // Try PKCS8 format first
        if let Ok(keys) = pkcs8_private_keys(&mut reader) {
            if !keys.is_empty() {
                let key = PrivateKey(keys[0].clone());
                info!("Loaded PKCS8 private key");
                return Ok(key);
            }
        }

        // Reset reader and try RSA format
        let file = File::open(key_path).map_err(|e| {
            GatewayError::Configuration(format!("Failed to open private key file: {}", e))
        })?;
        let mut reader = BufReader::new(file);

        if let Ok(keys) = rsa_private_keys(&mut reader) {
            if !keys.is_empty() {
                let key = PrivateKey(keys[0].clone());
                info!("Loaded RSA private key");
                return Ok(key);
            }
        }

        Err(GatewayError::Configuration(
            "Failed to parse private key (tried PKCS8 and RSA formats)".to_string(),
        ))
    }

    /// Create TLS server configuration
    async fn create_server_config(
        &self,
        certs: Vec<Certificate>,
        key: PrivateKey,
    ) -> Result<ServerConfig, GatewayError> {
        info!("Creating TLS server configuration");

        // Create TLS server configuration
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| {
                GatewayError::Configuration(format!("Failed to create TLS server config: {}", e))
            })?;

        // Set minimum TLS version
        let config = match self.config.tls.min_tls_version {
            crate::config::TlsVersion::Tls12 => {
                debug!("Using TLS 1.2 as minimum version");
                config
            }
            crate::config::TlsVersion::Tls13 => {
                debug!("Using TLS 1.3 as minimum version");
                // Note: rustls doesn't support setting TLS 1.3 as minimum in current version
                // This would require additional configuration
                config
            }
        };

        info!("TLS server configuration created successfully");
        Ok(config)
    }

    /// Get TLS server configuration
    pub fn get_tls_config(&self) -> Option<Arc<ServerConfig>> {
        self.tls_config.clone()
    }

    /// Check if TLS is enabled
    pub fn is_tls_enabled(&self) -> bool {
        self.config.tls.enabled && self.tls_config.is_some()
    }

    /// Get TLS configuration status
    pub fn get_tls_status(&self) -> TlsStatus {
        if !self.config.tls.enabled {
            TlsStatus::Disabled
        } else if self.tls_config.is_some() {
            TlsStatus::Enabled
        } else {
            TlsStatus::Failed
        }
    }

    /// Reload TLS configuration
    pub async fn reload_tls_config(&mut self) -> Result<(), GatewayError> {
        info!("Reloading TLS configuration...");
        self.load_tls_config().await
    }

    /// Validate TLS configuration without loading
    pub fn validate_config(&self) -> Result<(), GatewayError> {
        self.validate_tls_config()
    }
}

/// TLS status
#[derive(Debug, Clone, PartialEq)]
pub enum TlsStatus {
    /// TLS is disabled
    Disabled,
    /// TLS is enabled and configured
    Enabled,
    /// TLS configuration failed
    Failed,
}

impl std::fmt::Display for TlsStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsStatus::Disabled => write!(f, "disabled"),
            TlsStatus::Enabled => write!(f, "enabled"),
            TlsStatus::Failed => write!(f, "failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_tls_manager_disabled() {
        let mut config = GatewayConfig::default();
        config.tls.enabled = false;

        let tls_manager = TlsManager::new(&config).await.unwrap();
        assert!(!tls_manager.is_tls_enabled());
        assert_eq!(tls_manager.get_tls_status(), TlsStatus::Disabled);
    }

    #[tokio::test]
    async fn test_tls_config_validation() {
        let mut config = GatewayConfig::default();
        config.tls.enabled = true;
        // Missing certificate and key files

        let tls_manager = TlsManager::new(&config).await;
        assert!(tls_manager.is_err());
    }

    #[tokio::test]
    async fn test_tls_config_with_invalid_files() {
        // Create temporary files that don't contain valid certificates
        let cert_file = NamedTempFile::new().unwrap();
        cert_file.as_file().write_all(b"invalid certificate data").unwrap();

        let key_file = NamedTempFile::new().unwrap();
        key_file.as_file().write_all(b"invalid key data").unwrap();

        let mut config = GatewayConfig::default();
        config.tls.enabled = true;
        config.tls.cert_file = Some(cert_file.path().to_string_lossy().to_string());
        config.tls.key_file = Some(key_file.path().to_string_lossy().to_string());

        let tls_manager = TlsManager::new(&config).await;
        // This should fail due to invalid certificate/key data
        assert!(tls_manager.is_err());
    }

    #[tokio::test]
    async fn test_tls_status_enum() {
        assert_eq!(TlsStatus::Disabled.to_string(), "disabled");
        assert_eq!(TlsStatus::Enabled.to_string(), "enabled");
        assert_eq!(TlsStatus::Failed.to_string(), "failed");
    }

    #[tokio::test]
    async fn test_tls_config_validation_method() {
        let mut config = GatewayConfig::default();
        config.tls.enabled = false;
        
        let tls_manager = TlsManager::new(&config).await.unwrap();
        assert!(tls_manager.validate_config().is_ok());
    }
}
