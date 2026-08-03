//! Collect text and spooled file parts from axum [`Multipart`].
//!
//! Used by [`super::HtmlForm::from_multipart`] to walk the body once and produce
//! a [`MultipartParts`] value for serde assembly.

use std::collections::HashMap;

use axum::extract::Multipart;

use super::{FormError, upload::spool_field, upload::UploadedFile};

/// Text fields and uploaded files from one multipart walk.
#[derive(Default)]
pub struct MultipartParts {
    pub text: HashMap<String, Vec<String>>,
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
            let _ = field.bytes().await;
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
            let _ = field.bytes().await;
        } else {
            let value = field
                .text()
                .await
                .map_err(|e| FormError::Multipart(e.to_string()))?;
            parts.text.entry(name).or_default().push(value);
        }
    }
    Ok(parts)
}

/// Deserialize a flat text map into a struct via JSON bridging.
///
/// Use in custom [`super::HtmlForm::assemble_submit`] when you need typed
/// deserializers (`form_vec_i64`, etc.) on text fields only.
pub fn deserialize_text_map<T: serde::de::DeserializeOwned>(
    text: &HashMap<String, Vec<String>>,
) -> Result<T, FormError> {
    let map: serde_json::Map<String, serde_json::Value> = text
        .iter()
        .map(|(k, v)| {
            let value = if v.is_empty() {
                serde_json::Value::String(String::new())
            } else if v.len() == 1 {
                serde_json::Value::String(v[0].clone())
            } else {
                serde_json::Value::Array(
                    v.iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                )
            };
            (k.clone(), value)
        })
        .collect();
    serde_json::from_value(serde_json::Value::Object(map))
        .map_err(|e| FormError::Deserialize(e.to_string()))
}
