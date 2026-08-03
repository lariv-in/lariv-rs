//! Blob storage backends for uploaded files.
//!
//! Only the local filesystem backend is implemented. Selecting `storageBackend = "gcs"`
//! in config does not fail startup (see [`crate::hooks::AttachState`] for
//! [`super::StateHook`]); instead every
//! [`Filestore`] operation on [`UnimplementedFilestore`] returns
//! [`FilestoreError::NotImplemented`], surfacing the misconfiguration as an ordinary
//! error the first time a filesystem operation is attempted.

use std::io;
use std::path::PathBuf;

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
/// config load (`local` → [`LocalFilestore`], `gcs` → [`UnimplementedFilestore`]).
pub type DynFilestore = dyn Filestore;

/// Local disk-backed [`Filestore`]. Relative `base_dir` resolves next to the
/// process current working directory.
pub struct LocalFilestore {
    base_dir: PathBuf,
}

impl LocalFilestore {
    pub fn new(configured_dir: impl Into<String>) -> Self {
        let configured = configured_dir.into();
        let dir = if configured.trim().is_empty() {
            "filesystem".to_string()
        } else {
            configured
        };
        let path = PathBuf::from(&dir);
        let base_dir = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&path)
        };
        Self { base_dir }
    }

    async fn ensure_base_dir(&self) -> Result<&PathBuf, FilestoreError> {
        tokio::fs::create_dir_all(&self.base_dir).await?;
        Ok(&self.base_dir)
    }
}

#[async_trait::async_trait]
impl Filestore for LocalFilestore {
    async fn save_from_reader(
        &self,
        reader: &mut (dyn AsyncRead + Send + Unpin),
        ext: &str,
    ) -> Result<String, FilestoreError> {
        let dir = self.ensure_base_dir().await?;
        let filename = format!("store-{}{}", uuid::Uuid::new_v4(), ext);
        let path = dir.join(&filename);
        let mut file = tokio::fs::File::create(&path).await?;
        tokio::io::copy(reader, &mut file).await?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn open(&self, path: &str, name: &str) -> Result<FileDownload, FilestoreError> {
        let file = tokio::fs::File::open(path).await?;
        let meta = file.metadata().await?;
        let content_type = mime_guess::from_path(name)
            .first_or_octet_stream()
            .to_string();
        Ok(FileDownload {
            filename: name.to_string(),
            content_type,
            size: meta.len(),
            reader: Box::new(file),
        })
    }

    async fn delete(&self, path: &str) -> Result<(), FilestoreError> {
        if path.is_empty() {
            return Ok(());
        }
        match tokio::fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn stored_size(&self, path: &str) -> Result<u64, FilestoreError> {
        if path.is_empty() {
            return Ok(0);
        }
        let meta = tokio::fs::metadata(path).await?;
        Ok(meta.len())
    }
}

/// Placeholder used when `storageBackend = "gcs"` is configured. GCS support is not
/// ported; every operation fails with [`FilestoreError::NotImplemented`] instead of
/// panicking at startup.
pub struct UnimplementedFilestore;

const GCS_NOT_IMPLEMENTED: &str =
    "p_filesystem: storageBackend \"gcs\" is not implemented in lariv-rs; only \"local\" is supported";

#[async_trait::async_trait]
impl Filestore for UnimplementedFilestore {
    async fn save_from_reader(
        &self,
        _reader: &mut (dyn AsyncRead + Send + Unpin),
        _ext: &str,
    ) -> Result<String, FilestoreError> {
        Err(FilestoreError::NotImplemented(GCS_NOT_IMPLEMENTED.into()))
    }

    async fn open(&self, _path: &str, _name: &str) -> Result<FileDownload, FilestoreError> {
        Err(FilestoreError::NotImplemented(GCS_NOT_IMPLEMENTED.into()))
    }

    async fn delete(&self, _path: &str) -> Result<(), FilestoreError> {
        Err(FilestoreError::NotImplemented(GCS_NOT_IMPLEMENTED.into()))
    }

    async fn stored_size(&self, _path: &str) -> Result<u64, FilestoreError> {
        Err(FilestoreError::NotImplemented(GCS_NOT_IMPLEMENTED.into()))
    }
}

/// Human-readable size label: `"12.3 MB"`, `"512.0 B"`, etc.
pub fn human_readable_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    for (i, unit) in UNITS.iter().enumerate() {
        if value < 1024.0 || i == UNITS.len() - 1 {
            return format!("{value:.1} {unit}");
        }
        value /= 1024.0;
    }
    "-".to_string()
}