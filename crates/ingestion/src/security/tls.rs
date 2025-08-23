use bridge_core::BridgeResult;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;
use tracing::{error, info};

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
    pub ca_file: Option<PathBuf>,
    pub verify_client: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_file: PathBuf::from(""),
            key_file: PathBuf::from(""),
            ca_file: None,
            verify_client: false,
        }
    }
}

/// TLS manager for handling secure connections
pub struct TlsManager {
    config: TlsConfig,
    server_config: Option<Arc<ServerConfig>>,
}

impl TlsManager {
    /// Create new TLS manager
    pub fn new(config: TlsConfig) -> BridgeResult<Self> {
        Ok(Self {
            config,
            server_config: None,
        })
    }

    /// Initialize TLS configuration
    pub async fn initialize(&mut self) -> BridgeResult<()> {
        info!("Initializing TLS configuration");

        // Load certificate and private key
        let cert_file = std::fs::File::open(&self.config.cert_file).map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Failed to open cert file: {}", e))
        })?;
        let key_file = std::fs::File::open(&self.config.key_file).map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Failed to open key file: {}", e))
        })?;

        // Parse certificate
        let cert_chain = vec![Certificate(std::fs::read(&self.config.cert_file).map_err(
            |e| bridge_core::BridgeError::configuration(format!("Failed to read cert file: {}", e)),
        )?)];

        // Parse private key
        let key_data = std::fs::read(&self.config.key_file).map_err(|e| {
            bridge_core::BridgeError::configuration(format!("Failed to read key file: {}", e))
        })?;
        let private_key = PrivateKey(key_data);

        // Build server config
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| {
                bridge_core::BridgeError::configuration(format!(
                    "Failed to create TLS config: {}",
                    e
                ))
            })?;

        self.server_config = Some(Arc::new(config));

        info!("TLS configuration initialized successfully");
        Ok(())
    }

    /// Get server configuration
    pub fn get_server_config(&self) -> Option<Arc<ServerConfig>> {
        self.server_config.clone()
    }

    /// Check if TLS is enabled
    pub fn is_enabled(&self) -> bool {
        self.server_config.is_some()
    }

    /// Get TLS configuration
    pub fn get_config(&self) -> &TlsConfig {
        &self.config
    }

    /// Validate TLS configuration
    pub fn validate_config(&self) -> BridgeResult<()> {
        if !self.config.cert_file.exists() {
            return Err(bridge_core::BridgeError::configuration(format!(
                "Certificate file does not exist: {:?}",
                self.config.cert_file
            )));
        }

        if !self.config.key_file.exists() {
            return Err(bridge_core::BridgeError::configuration(format!(
                "Private key file does not exist: {:?}",
                self.config.key_file
            )));
        }

        if let Some(ref ca_file) = self.config.ca_file {
            if !ca_file.exists() {
                return Err(bridge_core::BridgeError::configuration(format!(
                    "CA file does not exist: {:?}",
                    ca_file
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert_eq!(config.cert_file, PathBuf::from(""));
        assert_eq!(config.key_file, PathBuf::from(""));
        assert!(config.ca_file.is_none());
        assert!(!config.verify_client);
    }

    #[test]
    fn test_tls_manager_creation() {
        let config = TlsConfig::default();
        let manager = TlsManager::new(config).unwrap();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_tls_config_validation() {
        let temp_dir = tempdir().unwrap();
        let cert_file = temp_dir.path().join("server.crt");
        let key_file = temp_dir.path().join("server.key");

        // Create dummy files
        fs::write(&cert_file, "dummy cert").unwrap();
        fs::write(&key_file, "dummy key").unwrap();

        let config = TlsConfig {
            cert_file,
            key_file,
            ca_file: None,
            verify_client: false,
        };

        let manager = TlsManager::new(config).unwrap();
        assert!(manager.validate_config().is_ok());
    }

    #[test]
    fn test_tls_config_validation_missing_files() {
        let config = TlsConfig::default();
        let manager = TlsManager::new(config).unwrap();
        assert!(manager.validate_config().is_err());
    }
}
