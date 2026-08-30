//! Persist inbound email attachments to the filesystem plugin (post-filter only).

use sea_orm::DatabaseConnection;

use crate::plugins::filesystem::{
    entities::VNode,
    node::{self, NodeFile},
    storage::DynFilestore,
};

use super::email_mime::ParsedAttachment;

const LOG_TARGET: &str = "llm_assistant::imap";

/// Save attachments under `{parent}/{uid}/`. Returns `(filename, vnode_id)` pairs.
pub async fn save_email_attachments(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
    uid: u32,
    attachments: &[ParsedAttachment],
) -> Vec<(String, i64)> {
    if attachments.is_empty() {
        return Vec::new();
    }

    let Some(parent_id) = parent_id.filter(|id| *id > 0) else {
        tracing::warn!(
            target: LOG_TARGET,
            uid,
            "email attachments folder not configured; skipping VNode save"
        );
        return Vec::new();
    };

    let parent = match node::get_by_id(db, parent_id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            tracing::warn!(target: LOG_TARGET, uid, "attachments parent {parent_id} not found");
            return Vec::new();
        }
        Err(e) => {
            tracing::warn!(target: LOG_TARGET, uid, "load attachments parent failed: {e}");
            return Vec::new();
        }
    };

    if !parent.is_directory {
        tracing::warn!(
            target: LOG_TARGET,
            uid,
            "attachments parent {parent_id} is not a directory"
        );
        return Vec::new();
    }

    let folder_name = uid.to_string();
    let subfolder = match ensure_subfolder(db, store, &parent, &folder_name).await {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!(target: LOG_TARGET, uid, "create attachment folder failed: {e}");
            return Vec::new();
        }
    };

    let mut saved = Vec::with_capacity(attachments.len());
    for att in attachments {
        let unique_name = unique_filename(db, subfolder.id, &att.filename).await;
        match node::create(
            db,
            store,
            unique_name.clone(),
            false,
            Some(NodeFile::Bytes {
                filename: unique_name.clone(),
                data: att.bytes.clone(),
            }),
            Some(&subfolder),
        )
        .await
        {
            Ok(vnode) => saved.push((att.filename.clone(), vnode.id)),
            Err(e) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    uid,
                    file = %att.filename,
                    "save attachment failed: {e}"
                );
            }
        }
    }
    saved
}

async fn ensure_subfolder(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent: &VNode,
    name: &str,
) -> Result<VNode, node::NodeError> {
    if let Some(existing) = node::find_child(db, Some(parent.id), name, true).await? {
        return Ok(existing);
    }
    node::create(db, store, name.to_string(), true, None, Some(parent)).await
}

async fn unique_filename(db: &DatabaseConnection, parent_id: i64, base: &str) -> String {
    if node::find_child(db, Some(parent_id), base, false)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return base.to_string();
    }

    let path = std::path::Path::new(base);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(base);
    let ext = path.extension().and_then(|s| s.to_str());
    let ext_suffix = ext.map(|e| format!(".{e}")).unwrap_or_default();

    for n in 2..1000 {
        let candidate = format!("{stem}-{n}{ext_suffix}");
        if node::find_child(db, Some(parent_id), &candidate, false)
            .await
            .ok()
            .flatten()
            .is_none()
        {
            return candidate;
        }
    }
    format!("{stem}-{}", chrono::Utc::now().timestamp_millis())
}
