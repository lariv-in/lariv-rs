//! Blob storage for uploaded files.
//!
//! Concrete backends ([`super::storage::LocalFilestore`], [`super::storage::GcsFilestore`])
//! live next to this trait. Request contexts in `lariv-core` cannot name this plugin, so
//! they hold the handle from [`erase_store`] and callers recover it with [`ErasedFilestore`].

use std::any::Any;
use std::io;
use std::sync::Arc;

use tokio::io::AsyncRead;

/// A file ready to be streamed back to an HTTP client.
pub struct FileDownload {
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub reader: Box<dyn AsyncRead + Send + Unpin>,
}

#[derive(Debug)]
pub enum FilestoreError {
    Io(io::Error),
    NotImplemented(String),
}

impl std::fmt::Display for FilestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::NotImplemented(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for FilestoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::NotImplemented(_) => None,
        }
    }
}

impl From<io::Error> for FilestoreError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl FilestoreError {
    /// true when the backing blob is absent.
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Io(e) if e.kind() == io::ErrorKind::NotFound)
    }
}

/// Persists uploaded files and serves them back by an opaque path string
/// returned from `save`/`save_from_reader`.
///
/// `Send + Sync` so backends can live behind [`DynFilestore`] in shared state.
#[async_trait::async_trait]
pub trait Filestore: Send + Sync {
    async fn save_from_reader(
        &self,
        reader: &mut (dyn AsyncRead + Send + Unpin),
        ext: &str,
    ) -> Result<String, FilestoreError>;

    async fn save(&self, data: &[u8], ext: &str) -> Result<String, FilestoreError> {
        let mut cursor = std::io::Cursor::new(data);
        self.save_from_reader(&mut cursor, ext).await
    }

    async fn open(&self, path: &str, name: &str) -> Result<FileDownload, FilestoreError>;

    async fn delete(&self, path: &str) -> Result<(), FilestoreError>;

    async fn stored_size(&self, path: &str) -> Result<u64, FilestoreError>;
}

/// Config-selected [`Filestore`] (`storageBackend` in `[filesystem]`).
///
/// Dynamic dispatch is required: the concrete backend is only known after
/// config load.
pub type DynFilestore = dyn Filestore;

/// Place a filestore on a request context that cannot name this plugin.
pub fn erase_store(store: Arc<DynFilestore>) -> Arc<dyn Any + Send + Sync> {
    Arc::new(store)
}

/// Recover a filestore stored with [`erase_store`].
pub trait ErasedFilestore {
    fn filestore(&self) -> &DynFilestore;
    fn filestore_arc(&self) -> Arc<DynFilestore>;
}

impl ErasedFilestore for Arc<dyn Any + Send + Sync> {
    fn filestore(&self) -> &DynFilestore {
        self.downcast_ref::<Arc<DynFilestore>>()
            .expect("context store is not a filestore")
            .as_ref()
    }

    fn filestore_arc(&self) -> Arc<DynFilestore> {
        Arc::clone(
            self.downcast_ref::<Arc<DynFilestore>>()
                .expect("context store is not a filestore"),
        )
    }
}

/// Always-erroring stub for unit tests that need a [`DynFilestore`] without I/O.
pub struct UnimplementedFilestore;

#[async_trait::async_trait]
impl Filestore for UnimplementedFilestore {
    async fn save_from_reader(
        &self,
        _reader: &mut (dyn AsyncRead + Send + Unpin),
        _ext: &str,
    ) -> Result<String, FilestoreError> {
        Err(FilestoreError::NotImplemented(
            "UnimplementedFilestore: no backend configured".into(),
        ))
    }

    async fn open(&self, _path: &str, _name: &str) -> Result<FileDownload, FilestoreError> {
        Err(FilestoreError::NotImplemented(
            "UnimplementedFilestore: no backend configured".into(),
        ))
    }

    async fn delete(&self, _path: &str) -> Result<(), FilestoreError> {
        Err(FilestoreError::NotImplemented(
            "UnimplementedFilestore: no backend configured".into(),
        ))
    }

    async fn stored_size(&self, _path: &str) -> Result<u64, FilestoreError> {
        Err(FilestoreError::NotImplemented(
            "UnimplementedFilestore: no backend configured".into(),
        ))
    }
}
