//! Signing key backends. The private key never lives in the database.

use std::sync::Arc;

use async_trait::async_trait;

use super::config::{SigningBackend, SigningConfig};

pub mod kms;
pub mod local;

use kms::KmsKeys;
use local::LocalKeys;

#[derive(Debug)]
pub struct KeyError(pub String);

impl KeyError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for KeyError {}

impl From<std::io::Error> for KeyError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

/// Create a P-256 signing key and sign SHA-256 digests.
///
/// `certificate_der` builds a signer certificate while signing. It is not stored.
#[async_trait]
pub trait KeyBackend: Send + Sync {
    async fn create_key(&self, user_id: i64) -> Result<String, KeyError>;

    /// DER-encoded ECDSA signature over `digest` (the digest itself, not the raw message).
    async fn sign_digest(&self, key_ref: &str, digest: &[u8]) -> Result<Vec<u8>, KeyError>;

    async fn certificate_der(
        &self,
        key_ref: &str,
        identity: &super::cert::SignerIdentity,
    ) -> Result<Vec<u8>, KeyError>;

    /// Local disk storage is for debugging only.
    fn debug_local(&self) -> bool {
        false
    }
}

pub type DynKeyBackend = dyn KeyBackend;

/// Build the backend selected by [`SigningConfig`].
///
/// Panics if `backend = "kms"` and project, location, or key ring is missing.
pub fn key_backend_from_config(config: &SigningConfig) -> Arc<DynKeyBackend> {
    match config.backend {
        SigningBackend::Local => {
            tracing::warn!(
                dir = %config.local_dir,
                "signing local backend stores private keys on disk; use only for debugging"
            );
            Arc::new(LocalKeys::new(config.local_dir.clone()))
        }
        SigningBackend::Kms => {
            if !config.kms_ready() {
                panic!("signing backend kms requires kmsProject, kmsLocation, and kmsKeyRing");
            }
            Arc::new(KmsKeys::new(config.clone()))
        }
    }
}
