//! Signing plugin configuration (`[signing]` in TOML).

use serde::{Deserialize, Serialize};

use lariv_core::config::ConfigSection;

/// Config HList tag for [`SigningConfig`].
pub struct SigningConfigTag;

impl ConfigSection for SigningConfigTag {
    const KEY: Option<&'static str> = Some("signing");
}

/// Key backend selector (`local` or `kms`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SigningBackend {
    #[default]
    Local,
    Kms,
}

fn default_local_dir() -> String {
    "signing-keys".into()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SigningConfig {
    #[serde(default, rename = "backend")]
    pub backend: SigningBackend,
    #[serde(default = "default_local_dir", rename = "localDir")]
    pub local_dir: String,
    /// GCP project id (required when `backend` is `"kms"`).
    #[serde(default, rename = "kmsProject")]
    pub kms_project: String,
    /// Cloud KMS location, such as `global`.
    #[serde(default, rename = "kmsLocation")]
    pub kms_location: String,
    /// Cloud KMS key ring that holds per-user signing keys.
    #[serde(default, rename = "kmsKeyRing")]
    pub kms_key_ring: String,
    /// Path to a service account JSON key file. Empty uses Application Default Credentials.
    #[serde(default, rename = "kmsCredentialsFile")]
    pub kms_credentials_file: String,
}

impl Default for SigningConfig {
    fn default() -> Self {
        Self {
            backend: SigningBackend::default(),
            local_dir: default_local_dir(),
            kms_project: String::new(),
            kms_location: String::new(),
            kms_key_ring: String::new(),
            kms_credentials_file: String::new(),
        }
    }
}

impl SigningConfig {
    pub fn kms_ready(&self) -> bool {
        !self.kms_project.trim().is_empty()
            && !self.kms_location.trim().is_empty()
            && !self.kms_key_ring.trim().is_empty()
    }
}
