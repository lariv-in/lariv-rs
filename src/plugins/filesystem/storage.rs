//! Blob storage backends for uploaded files.
//!
//! Configures [`LocalFilestore`] or [`GcsFilestore`] from `[filesystem]`
//! (`storageBackend`, `localDir`, `gcsBucket`, …).

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use futures_util::TryStreamExt;
use object_store::Attribute;
use object_store::Attributes;
use object_store::ObjectStore;
use object_store::buffered::BufWriter;
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::path::Path as ObjectPath;
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio_util::io::StreamReader;

use super::config::{FilesystemConfig, StorageBackend};

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

impl From<object_store::Error> for FilestoreError {
    fn from(value: object_store::Error) -> Self {
        match value {
            object_store::Error::NotFound { .. } => {
                Self::Io(io::Error::new(io::ErrorKind::NotFound, value.to_string()))
            }
            other => Self::Io(io::Error::other(other)),
        }
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
/// config load (`local` → [`LocalFilestore`], `gcs` → [`GcsFilestore`]).
pub type DynFilestore = dyn Filestore;

/// Build the filestore selected by [`FilesystemConfig`].
///
/// Panics if `storageBackend = "gcs"` and the bucket is missing or the GCS
/// client cannot be constructed (same fail-fast behavior as Go `PostConfig`).
pub fn filestore_from_config(config: &FilesystemConfig) -> Arc<DynFilestore> {
    match config.storage_backend {
        StorageBackend::Local => Arc::new(LocalFilestore::new(config.local_dir.clone())),
        StorageBackend::Gcs => Arc::new(
            GcsFilestore::new(
                &config.gcs_bucket,
                &config.gcs_credentials_file,
                &config.gcs_prefix,
            )
            .unwrap_or_else(|e| panic!("failed to initialize GCS filestore: {e}")),
        ),
    }
}

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

/// Google Cloud Storage–backed [`Filestore`].
///
/// The string returned from save is the object key within the configured bucket
/// (including [`Self::prefix`]), matching Go `GCSFilestore`.
pub struct GcsFilestore {
    store: Arc<dyn ObjectStore>,
    prefix: String,
}

impl GcsFilestore {
    /// Build a GCS-backed store. Empty `credentials_file` uses Application Default
    /// Credentials (metadata server / `GOOGLE_APPLICATION_CREDENTIALS` / gcloud).
    /// `prefix` is normalized to a non-empty path ending with `/`.
    pub fn new(
        bucket_name: &str,
        credentials_file: &str,
        prefix: &str,
    ) -> Result<Self, FilestoreError> {
        let bucket_name = bucket_name.trim();
        if bucket_name.is_empty() {
            return Err(FilestoreError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "gcs bucket name is required",
            )));
        }

        let mut builder = GoogleCloudStorageBuilder::new().with_bucket_name(bucket_name);
        let credentials_file = credentials_file.trim();
        if !credentials_file.is_empty() {
            builder = builder.with_service_account_path(credentials_file);
        }

        let store = builder.build().map_err(|e| {
            tracing::error!(bucket = bucket_name, error = %e, "failed creating GCS storage client");
            FilestoreError::Io(io::Error::other(e))
        })?;

        Ok(Self {
            store: Arc::new(store),
            prefix: normalize_gcs_prefix(prefix),
        })
    }

    fn new_object_key(&self, ext: &str) -> String {
        format!("{}{}{}", self.prefix, uuid::Uuid::new_v4().simple(), ext)
    }
}

fn normalize_gcs_prefix(p: &str) -> String {
    let p = p.trim();
    if p.is_empty() {
        return "lariv/".into();
    }
    let p = p.trim_start_matches('/');
    if p.ends_with('/') {
        p.to_string()
    } else {
        format!("{p}/")
    }
}

#[async_trait::async_trait]
impl Filestore for GcsFilestore {
    async fn save_from_reader(
        &self,
        reader: &mut (dyn AsyncRead + Send + Unpin),
        ext: &str,
    ) -> Result<String, FilestoreError> {
        let key = self.new_object_key(ext);
        let path = ObjectPath::from(key.as_str());
        let content_type = mime_guess::from_ext(ext.trim_start_matches('.'))
            .first_or_octet_stream()
            .to_string();
        let mut attrs = Attributes::new();
        attrs.insert(Attribute::ContentType, content_type.into());

        let mut writer =
            BufWriter::new(Arc::clone(&self.store), path.clone()).with_attributes(attrs);
        if let Err(e) = tokio::io::copy(reader, &mut writer).await {
            let _ = writer.abort().await;
            tracing::error!(key = %key, error = %e, "failed uploading to GCS");
            let _ = self.store.delete(&path).await;
            return Err(e.into());
        }
        if let Err(e) = writer.shutdown().await {
            tracing::error!(key = %key, error = %e, "failed closing GCS writer");
            let _ = self.store.delete(&path).await;
            return Err(FilestoreError::Io(io::Error::other(e)));
        }
        Ok(key)
    }

    async fn open(&self, path: &str, name: &str) -> Result<FileDownload, FilestoreError> {
        if path.is_empty() {
            return Err(FilestoreError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "empty object key",
            )));
        }
        let object_path = ObjectPath::from(path);
        let get = self.store.get(&object_path).await.map_err(|e| {
            tracing::error!(key = path, error = %e, "failed opening GCS object");
            FilestoreError::from(e)
        })?;
        let size = get.meta.size;
        let content_type = get
            .attributes
            .get(&Attribute::ContentType)
            .map(|v| v.to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                mime_guess::from_path(name)
                    .first_or_octet_stream()
                    .to_string()
            });
        let stream = get.into_stream().map_err(io::Error::other);
        let reader = StreamReader::new(stream);
        Ok(FileDownload {
            filename: name.to_string(),
            content_type,
            size,
            reader: Box::new(reader),
        })
    }

    async fn delete(&self, path: &str) -> Result<(), FilestoreError> {
        if path.is_empty() {
            return Ok(());
        }
        match self.store.delete(&ObjectPath::from(path)).await {
            Ok(()) => Ok(()),
            Err(object_store::Error::NotFound { .. }) => {
                tracing::warn!(key = path, "GCS object already missing during delete");
                Ok(())
            }
            Err(e) => {
                tracing::error!(key = path, error = %e, "failed deleting GCS object");
                Err(e.into())
            }
        }
    }

    async fn stored_size(&self, path: &str) -> Result<u64, FilestoreError> {
        if path.is_empty() {
            return Ok(0);
        }
        let meta = self.store.head(&ObjectPath::from(path)).await?;
        Ok(meta.size)
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
