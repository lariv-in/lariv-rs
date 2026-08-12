//! Filesystem plugin configuration (`[filesystem]` in TOML).

use serde::{Deserialize, Serialize};

use crate::config::ConfigSection;

/// Config HList tag for [`FilesystemConfig`].
pub struct FilesystemConfigTag;

impl ConfigSection for FilesystemConfigTag {
    const KEY: Option<&'static str> = Some("filesystem");
}

/// Storage backend selector (`local` or `gcs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    #[default]
    Local,
    Gcs,
}

fn default_local_dir() -> String {
    "filesystem".into()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilesystemConfig {
    #[serde(default, rename = "storageBackend")]
    pub storage_backend: StorageBackend,
    #[serde(default = "default_local_dir", rename = "localDir")]
    pub local_dir: String,
    /// GCS bucket name (required when `storageBackend` is `"gcs"`).
    #[serde(default, rename = "gcsBucket")]
    pub gcs_bucket: String,
    /// Path to a service account JSON key file. Empty uses Application Default Credentials.
    #[serde(default, rename = "gcsCredentialsFile")]
    pub gcs_credentials_file: String,
    /// Object key prefix (default `lariv/`). Normalized to end with `/`.
    #[serde(default, rename = "gcsPrefix")]
    pub gcs_prefix: String,
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        Self {
            storage_backend: StorageBackend::default(),
            local_dir: default_local_dir(),
            gcs_bucket: String::new(),
            gcs_credentials_file: String::new(),
            gcs_prefix: String::new(),
        }
    }
}
