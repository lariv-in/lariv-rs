//! Persist chat uploads under `{chat_attachments_parent}/{session_id}/`.
//!
//! Message attachment VNode ids are recorded on part rows (`vnode_id`) so tools
//! can list attachments from conversation history rather than from the folder.

use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, JoinType, QueryFilter, QuerySelect,
    RelationTrait,
};

use crate::plugins::filesystem::{
    entities::VNode,
    node::{self, NodeError},
    storage::DynFilestore,
};

use super::{
    entities::{part_file_data, part_inline_data, session_message, session_message_part},
    preferences::{load_preferences, save_preferences},
};

const LOG_TARGET: &str = "llm_assistant::chat_attachments";
pub const DEFAULT_CHAT_ATTACHMENTS_DIR: &str = "chat_attachments";

/// One attachment VNode referenced from a session message part.
#[derive(Debug, Clone)]
pub struct SessionAttachment {
    pub vnode_id: i64,
    pub display_name: String,
}

/// Ensure `/chat_attachments` exists and is stored on preferences when unset.
pub async fn ensure_chat_attachments_parent(
    db: &DatabaseConnection,
    store: &DynFilestore,
) -> Result<VNode, NodeError> {
    let prefs = load_preferences(db)
        .await
        .map_err(NodeError::Db)?;

    if let Some(parent_id) = prefs.chat_attachments_parent_id.filter(|id| *id > 0) {
        match node::get_by_id(db, parent_id).await.map_err(NodeError::Db)? {
            Some(v) if v.is_directory => return Ok(v),
            Some(_) => {
                tracing::warn!(
                    target: LOG_TARGET,
                    parent_id,
                    "chat attachments parent is not a directory; recreating default"
                );
            }
            None => {
                tracing::warn!(
                    target: LOG_TARGET,
                    parent_id,
                    "chat attachments parent missing; recreating default"
                );
            }
        }
    }

    let parent_id = node::ensure_directory_path(
        db,
        store,
        None,
        &[DEFAULT_CHAT_ATTACHMENTS_DIR.to_string()],
    )
    .await?
    .ok_or_else(|| NodeError::Validation("failed to create chat_attachments".into()))?;

    let parent = node::get_by_id(db, parent_id)
        .await
        .map_err(NodeError::Db)?
        .ok_or_else(|| NodeError::Validation("chat_attachments not found after create".into()))?;

    let mut prefs = prefs;
    prefs.chat_attachments_parent_id = Some(parent.id);
    save_preferences(db, prefs)
        .await
        .map_err(NodeError::Db)?;

    Ok(parent)
}

/// Find or create `{parent}/{session_id}/` for a conversation's uploads.
pub async fn ensure_conversation_folder(
    db: &DatabaseConnection,
    store: &DynFilestore,
    session_id: i64,
) -> Result<VNode, NodeError> {
    let parent = ensure_chat_attachments_parent(db, store).await?;
    let folder_name = session_id.to_string();
    if let Some(existing) = node::find_child(db, Some(parent.id), &folder_name, true)
        .await
        .map_err(NodeError::Db)?
    {
        return Ok(existing);
    }
    node::create(db, store, folder_name, true, None, Some(&parent)).await
}

/// Collect attachment VNode refs from the session's message parts (`inline_data` + `file_data`).
///
/// Dedupes by `vnode_id` (first occurrence wins). Does not require the conversation folder.
pub async fn list_session_attachment_refs(
    db: &DatabaseConnection,
    session_id: i64,
) -> Result<Vec<SessionAttachment>, DbErr> {
    let inline_rows = part_inline_data::Entity::find()
        .join(
            JoinType::InnerJoin,
            part_inline_data::Relation::Part.def(),
        )
        .join(
            JoinType::InnerJoin,
            session_message_part::Relation::Message.def(),
        )
        .filter(session_message::Column::LlmAssistantSessionId.eq(session_id))
        .filter(part_inline_data::Column::VnodeId.is_not_null())
        .all(db)
        .await?;

    let file_rows = part_file_data::Entity::find()
        .join(JoinType::InnerJoin, part_file_data::Relation::Part.def())
        .join(
            JoinType::InnerJoin,
            session_message_part::Relation::Message.def(),
        )
        .filter(session_message::Column::LlmAssistantSessionId.eq(session_id))
        .filter(part_file_data::Column::VnodeId.is_not_null())
        .all(db)
        .await?;

    let mut entries: Vec<(i64, i64, String)> = Vec::new();
    for row in inline_rows {
        let Some(vid) = row.vnode_id.filter(|id| *id > 0) else {
            continue;
        };
        entries.push((
            row.llm_assistant_session_message_part_id,
            vid,
            row.display_name.unwrap_or_default(),
        ));
    }
    for row in file_rows {
        let Some(vid) = row.vnode_id.filter(|id| *id > 0) else {
            continue;
        };
        entries.push((
            row.llm_assistant_session_message_part_id,
            vid,
            row.display_name.unwrap_or_default(),
        ));
    }
    entries.sort_by_key(|(part_id, _, _)| *part_id);

    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (_, vnode_id, display_name) in entries {
        if !seen.insert(vnode_id) {
            continue;
        }
        out.push(SessionAttachment {
            vnode_id,
            display_name,
        });
    }
    Ok(out)
}
