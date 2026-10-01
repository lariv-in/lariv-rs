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
///
/// Fields whose names are listed in `file_names` / `multi_file_names` are always
/// treated as file parts when a filename is present (including after HTMX FormData).
/// Empty file inputs (`filename=""`) are discarded. Text parts that reuse a known
/// file field name are ignored so they cannot silently replace an upload.
pub async fn collect_multipart(
    mut multipart: Multipart,
    file_names: &[&str],
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
        let is_declared_file =
            file_names.contains(&name.as_str()) || multi_file_names.contains(&name.as_str());
        let has_filename = field.file_name().is_some_and(|n| !n.is_empty());
        if has_filename {
            let uploaded = spool_field(field).await?;
            if multi_file_names.contains(&name.as_str()) {
                parts.file_lists.entry(name).or_default().push(uploaded);
            } else {
                parts.files.insert(name, uploaded);
            }
        } else if field.file_name().is_some() || is_declared_file {
            // Empty file input, or a declared file field without a filename — discard.
            // Do not treat as text (that would hide a dropped upload as a string value).
            if let Err(e) = field.bytes().await {
                tracing::warn!(error = %e, field = %name, "failed discarding empty multipart file field");
            } else if is_declared_file {
                tracing::warn!(
                    field = %name,
                    "multipart file field present without filename; upload discarded"
                );
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
