//! Local disk key store. For debugging only: private keys are files on disk.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use rcgen::{KeyPair, PKCS_ECDSA_P256_SHA256};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use super::super::cert::SignerIdentity;
use super::pem::{self_signed_cert, sign_pem_digest};
use super::{KeyBackend, KeyError};

pub struct LocalKeys {
    base_dir: PathBuf,
}

impl LocalKeys {
    pub fn new(configured_dir: impl Into<String>) -> Self {
        let configured = configured_dir.into();
        let dir = if configured.trim().is_empty() {
            "signing-keys".to_string()
        } else {
            configured
        };
        let path = PathBuf::from(&dir);
        let base_dir = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        };
        Self { base_dir }
    }

    fn filename_for(user_id: i64) -> String {
        format!("user-{user_id}.pem")
    }

    fn key_path(&self, key_ref: &str) -> Result<PathBuf, KeyError> {
        if !valid_key_ref(key_ref) {
            return Err(KeyError::msg("invalid key reference"));
        }
        Ok(self.base_dir.join(key_ref))
    }

    async fn ensure_dir(&self) -> Result<&Path, KeyError> {
        tokio::fs::create_dir_all(&self.base_dir).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&self.base_dir, std::fs::Permissions::from_mode(0o700))
                .await?;
        }
        Ok(&self.base_dir)
    }

    async fn read_pem(&self, key_ref: &str) -> Result<String, KeyError> {
        let path = self.key_path(key_ref)?;
        tokio::fs::read_to_string(&path)
            .await
            .map_err(|err| KeyError::msg(format!("read signing key: {err}")))
    }
}

fn valid_key_ref(key_ref: &str) -> bool {
    !key_ref.is_empty()
        && !key_ref.contains('/')
        && !key_ref.contains('\\')
        && !key_ref.contains("..")
        && key_ref
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

#[async_trait]
impl KeyBackend for LocalKeys {
    async fn create_key(&self, user_id: i64) -> Result<String, KeyError> {
        self.ensure_dir().await?;
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .map_err(|err| KeyError::msg(format!("generate signing key: {err}")))?;
        let pem = key.serialize_pem();
        let filename = Self::filename_for(user_id);
        let path = self.key_path(&filename)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        let mut file = match options.open(&path).await {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => return Ok(filename),
            Err(err) => {
                return Err(KeyError::msg(format!("create signing key file: {err}")));
            }
        };
        file.write_all(pem.as_bytes())
            .await
            .map_err(|err| KeyError::msg(format!("write signing key: {err}")))?;
        file.flush().await?;
        Ok(filename)
    }

    async fn sign_digest(&self, key_ref: &str, digest: &[u8]) -> Result<Vec<u8>, KeyError> {
        let pem = self.read_pem(key_ref).await?;
        sign_pem_digest(&pem, digest)
    }

    async fn certificate_der(
        &self,
        key_ref: &str,
        identity: &SignerIdentity,
    ) -> Result<Vec<u8>, KeyError> {
        let pem = self.read_pem(key_ref).await?;
        self_signed_cert(&pem, identity)
    }

    fn debug_local(&self) -> bool {
        true
    }
}
