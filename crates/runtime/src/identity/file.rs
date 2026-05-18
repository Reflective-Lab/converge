//! File-based certificate identity provider.
//!
//! Loads certificates and keys from PEM files on disk.
//! Suitable for development, testing, and production without SPIFFE.

use super::{Identity, IdentityError};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

#[cfg(feature = "security")]
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
#[cfg(feature = "security")]
use rustls::{ClientConfig, RootCertStore, ServerConfig};
#[cfg(feature = "security")]
use rustls_pemfile::{certs as read_certs, private_key};

/// Configuration for file-based identity.
#[derive(Debug, Clone)]
pub struct FileIdentityConfig {
    /// Path to the service certificate (PEM format).
    pub cert_path: PathBuf,

    /// Path to the private key (PEM format).
    pub key_path: PathBuf,

    /// Path to CA certificate(s) for verifying peers (PEM format).
    pub ca_path: PathBuf,

    /// Service identifier extracted from certificate CN or provided explicitly.
    pub service_id: Option<String>,
}

impl FileIdentityConfig {
    /// Create a new file identity config.
    pub fn new(
        cert_path: impl Into<PathBuf>,
        key_path: impl Into<PathBuf>,
        ca_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            ca_path: ca_path.into(),
            service_id: None,
        }
    }

    /// Set an explicit service ID (otherwise extracted from cert CN).
    #[must_use]
    pub fn with_service_id(mut self, id: impl Into<String>) -> Self {
        self.service_id = Some(id.into());
        self
    }
}

/// File-based identity provider.
pub struct FileIdentity {
    service_id: String,
    #[cfg(feature = "security")]
    certs: Vec<CertificateDer<'static>>,
    #[cfg(feature = "security")]
    key: PrivateKeyDer<'static>,
    #[cfg(feature = "security")]
    root_store: RootCertStore,
}

impl FileIdentity {
    /// Create a new file-based identity from the given configuration.
    #[cfg(feature = "security")]
    pub fn new(config: FileIdentityConfig) -> Result<Self, IdentityError> {
        info!(
            cert_path = %config.cert_path.display(),
            key_path = %config.key_path.display(),
            ca_path = %config.ca_path.display(),
            "Loading file-based identity"
        );

        // Load certificate chain
        let cert_file = File::open(&config.cert_path).map_err(|e| {
            IdentityError::CertificateLoad(format!(
                "cannot open {}: {}",
                config.cert_path.display(),
                e
            ))
        })?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<CertificateDer<'static>> = read_certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| IdentityError::CertificateLoad(e.to_string()))?;

        if certs.is_empty() {
            return Err(IdentityError::NoCertificates);
        }
        debug!(count = certs.len(), "Loaded certificate chain");

        // Load private key
        let key_file = File::open(&config.key_path).map_err(|e| {
            IdentityError::PrivateKeyLoad(format!(
                "cannot open {}: {}",
                config.key_path.display(),
                e
            ))
        })?;
        let mut key_reader = BufReader::new(key_file);
        let key = private_key(&mut key_reader)
            .map_err(|e| IdentityError::PrivateKeyLoad(e.to_string()))?
            .ok_or(IdentityError::NoPrivateKey)?;
        debug!("Loaded private key");

        // Load CA certificates
        let ca_file = File::open(&config.ca_path).map_err(|e| {
            IdentityError::CaLoad(format!("cannot open {}: {}", config.ca_path.display(), e))
        })?;
        let mut ca_reader = BufReader::new(ca_file);
        let ca_certs: Vec<CertificateDer<'static>> = read_certs(&mut ca_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| IdentityError::CaLoad(e.to_string()))?;

        let mut root_store = RootCertStore::empty();
        for cert in ca_certs {
            root_store
                .add(cert)
                .map_err(|e| IdentityError::CaLoad(e.to_string()))?;
        }
        debug!(count = root_store.len(), "Loaded CA certificates");

        let service_id = config
            .service_id
            .unwrap_or_else(|| "unknown-service".to_string());
        info!(service_id = %service_id, "Identity loaded successfully");

        Ok(Self {
            service_id,
            certs,
            key,
            root_store,
        })
    }

    /// Create a placeholder identity when security feature is disabled.
    #[cfg(not(feature = "security"))]
    pub fn new(config: FileIdentityConfig) -> Result<Self, IdentityError> {
        let service_id = config
            .service_id
            .unwrap_or_else(|| "unknown-service".to_string());
        info!(service_id = %service_id, "Identity created (security disabled)");
        Ok(Self { service_id })
    }
}

#[cfg(feature = "security")]
impl Identity for FileIdentity {
    fn service_id(&self) -> &str {
        &self.service_id
    }

    fn client_config(&self) -> Result<Arc<ClientConfig>, IdentityError> {
        let config = ClientConfig::builder()
            .with_root_certificates(self.root_store.clone())
            .with_client_auth_cert(self.certs.clone(), self.key.clone_key())
            .map_err(|e| IdentityError::TlsConfig(e.to_string()))?;

        Ok(Arc::new(config))
    }

    fn server_config(&self) -> Result<Arc<ServerConfig>, IdentityError> {
        // Build client cert verifier that requires valid client certificates
        let client_cert_verifier =
            rustls::server::WebPkiClientVerifier::builder(Arc::new(self.root_store.clone()))
                .build()
                .map_err(|e| IdentityError::TlsConfig(e.to_string()))?;

        let config = ServerConfig::builder()
            .with_client_cert_verifier(client_cert_verifier)
            .with_single_cert(self.certs.clone(), self.key.clone_key())
            .map_err(|e| IdentityError::TlsConfig(e.to_string()))?;

        Ok(Arc::new(config))
    }
}

#[cfg(not(feature = "security"))]
impl Identity for FileIdentity {
    fn service_id(&self) -> &str {
        &self.service_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_identity_config() {
        let config =
            FileIdentityConfig::new("/path/to/cert.pem", "/path/to/key.pem", "/path/to/ca.pem")
                .with_service_id("test-service");

        assert_eq!(config.service_id, Some("test-service".to_string()));
    }
}
