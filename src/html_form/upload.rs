//! Typed multipart file fields: [`Upload`] (form marker) and [`UploadedFile`] (spooled body).
//!
//! Declare `Upload` on `#[html_form]` structs; handlers receive [`UploadedFile`] after
//! [`super::HtmlForm::from_multipart`]. Large files should use [`UploadedFile::into_reader`].

use std::path::PathBuf;

use bytes::Bytes;
/// Zero-sized marker used in `#[html_form]` definitions for file inputs.
///
/// Replaced by [`UploadedFile`] on [`crate::html_form::HtmlForm::Submit`] after
/// [`crate::html_form::HtmlForm::from_multipart`].
#[derive(Debug, Default, Clone, Copy)]
pub struct Upload;

/// One uploaded multipart file part, spooled to a temp path.
///
/// The temp file is removed on drop; consume via [`Self::into_reader`] or [`Self::into_bytes`].
#[derive(Debug)]
pub struct UploadedFile {
    pub filename: String,
    pub content_type: Option<String>,
    path: PathBuf,
}

impl UploadedFile {
    pub(crate) fn new(filename: String, content_type: Option<String>, path: PathBuf) -> Self {
        Self {
            filename,
            content_type,
            path,
        }
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Consume into an async reader over the spooled temp file.
    pub async fn into_reader(self) -> std::io::Result<tokio::fs::File> {
        tokio::fs::File::open(&self.path).await
    }

    /// Buffer the whole body (small files / tests). Prefer [`Self::into_reader`] for large uploads.
    pub async fn into_bytes(self) -> std::io::Result<Bytes> {
        let data = tokio::fs::read(&self.path).await?;
        Ok(Bytes::from(data))
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for UploadedFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Stream an axum multipart field into a tempfile.
///
/// Called by [`super::multipart::collect_multipart`]; rarely needed directly.
pub async fn spool_field(
    field: axum::extract::multipart::Field<'_>,
) -> Result<UploadedFile, super::FormError> {
    use futures_util::TryStreamExt;
    use tokio_util::io::StreamReader;

    let filename = field
        .file_name()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "upload.bin".into());
    let content_type = field.content_type().map(str::to_string);

    let tmp = tempfile::NamedTempFile::new()
        .map_err(|e| super::FormError::Spool(e.to_string()))?;
    let path = tmp
        .into_temp_path()
        .keep()
        .map_err(|e| super::FormError::Spool(e.to_string()))?;

    let stream = field.map_err(std::io::Error::other);
    let mut reader = StreamReader::new(stream);
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| super::FormError::Spool(e.to_string()))?;
    tokio::io::copy(&mut reader, &mut file)
        .await
        .map_err(|e| super::FormError::Spool(e.to_string()))?;

    Ok(UploadedFile::new(filename, content_type, path))
}
