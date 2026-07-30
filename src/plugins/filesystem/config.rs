//! Filesystem plugin configuration (`[p_filesystem]` in TOML), aligned with Go `p_filesystem.FilesystemConfig`.

use serde::{Deserialize, Serialize};

use crate::config::ConfigSection;

/// Config HList tag for [`FilesystemConfig`].
pub struct FilesystemConfigTag;

impl ConfigSection for FilesystemConfigTag {
    const KEY: Option<&'static str> = Some("p_filesystem");
}

/// Storage backend selector (Go `StorageBackend`). Only `Local` is implemented; `Gcs`
/// is accepted for config compatibility but falls back to an always-erroring store.
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
    /// GCS: bucket name (required when `storageBackend` is `"gcs"`). Unused — GCS unimplemented.
    #[serde(default, rename = "gcsBucket")]
    pub gcs_bucket: String,
    /// GCS: path to service account JSON key file. Unused — GCS unimplemented.
    #[serde(default, rename = "gcsCredentialsFile")]
    pub gcs_credentials_file: String,
    /// GCS: object key prefix. Unused — GCS unimplemented.
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
