//! Node file payloads and error type.

use sea_orm::DbErr;

use lariv_core::html_form::UploadedFile;

use crate::storage::{DynFilestore, FilestoreError};

/// File payload for [`create`] / [`update`] — bytes or a spooled multipart upload.
pub enum NodeFile {
    Bytes { filename: String, data: Vec<u8> },
    Upload(UploadedFile),
}

impl NodeFile {
    pub fn filename(&self) -> &str {
        match self {
            Self::Bytes { filename, .. } => filename,
            Self::Upload(u) => u.filename(),
        }
    }

    pub(crate) async fn save_to_store(&self, store: &DynFilestore) -> Result<String, NodeError> {
        let ext = super::text::ext_of(self.filename());
        match self {
            Self::Bytes { data, .. } => store.save(data, &ext).await.map_err(NodeError::Store),
            Self::Upload(u) => {
                // Re-open path without dropping the UploadedFile yet.
                let mut reader = tokio::fs::File::open(u.path())
                    .await
                    .map_err(|e| NodeError::Validation(e.to_string()))?;
                store
                    .save_from_reader(&mut reader, &ext)
                    .await
                    .map_err(NodeError::Store)
            }
        }
    }
}

#[derive(Debug)]
pub enum NodeError {
    Validation(String),
    Conflict,
    Forbidden,
    Db(DbErr),
    Store(FilestoreError),
}

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(msg) => write!(f, "{msg}"),
            Self::Conflict => write!(f, "an item with this name already exists here"),
            Self::Forbidden => write!(f, "you do not have access to this item"),
            Self::Db(e) => write!(f, "{e}"),
            Self::Store(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for NodeError {}

impl From<DbErr> for NodeError {
    fn from(value: DbErr) -> Self {
        Self::Db(value)
    }
}
