//! Collect text and spooled file parts from axum [`Multipart`].
//!
//! Used by [`super::HtmlForm::from_multipart`] to walk the body once and produce
//! a [`MultipartParts`] value for serde assembly.

use std::collections::HashMap;

use axum::extract::Multipart;

use super::{FormError, upload::UploadedFile, upload::spool_field, urlencoded::UrlencodedFields};

/// Text fields and uploaded files from one multipart walk.
#[derive(Default)]
pub struct MultipartParts {
    pub text: UrlencodedFields,
    pub files: HashMap<String, UploadedFile>,
    pub file_lists: HashMap<String, Vec<UploadedFile>>,
}

/// Walk multipart fields once, spooling file parts to temp storage.
///
/// Names in `multi_file_names` accumulate into [`MultipartParts::file_lists`];
/// other file parts go into [`MultipartParts::files`] (last wins).
pub async fn collect_multipart(
    mut multipart: Multipart,
    _file_names: &[&str],
    multi_file_names: &[&str],
) -> Result<MultipartParts, FormError> {
    let mut parts = MultipartParts::default();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| FormError::Multipart(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name.is_empty() {
            if let Err(e) = field.bytes().await {
                tracing::warn!(error = %e, "failed discarding unnamed multipart field");
            }
            continue;
        }
        let has_filename = field.file_name().is_some_and(|n| !n.is_empty());
        if has_filename {
            let uploaded = spool_field(field).await?;
            if multi_file_names.contains(&name.as_str()) {
                parts.file_lists.entry(name).or_default().push(uploaded);
            } else {
                parts.files.insert(name, uploaded);
            }
        } else if field.file_name().is_some() {
            // Empty file input — discard.
            if let Err(e) = field.bytes().await {
                tracing::warn!(error = %e, field = %name, "failed discarding empty multipart file field");
            }
        } else {
            let value = field
                .text()
                .await
                .map_err(|e| FormError::Multipart(e.to_string()))?;
            parts.text.push(name, value);
        }
    }
    Ok(parts)
}
