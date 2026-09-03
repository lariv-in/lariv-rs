//! Inbound email triage — MIME parse, LLM filter, Files API upload, assistant turn.

use std::sync::Arc;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::plugins::users::entities::user::Entity as UserEntity;

use super::{
    actions::run_stream_turn,
    content::attachments::upload_attachment_part,
    email_attachments::save_email_attachments,
    email_mime::{ParsedEmail, attachment_metadata_lines, parse_rfc822},
    entities::{
        processed_email::{self, Entity as ProcessedEmailEntity},
        session,
    },
    genai::{Content, GenaiClient, Part, Role},
    live_turn,
    preferences::load_preferences,
    state::LlmAssistantState,
};

const LOG_TARGET: &str = "llm_assistant::imap";
const FILTER_MAX_OUTPUT_TOKENS: i32 = 1024;
const FILTER_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "act": {
      "type": "boolean",
      "description": "True when the assistant should handle this email."
    },
    "reason": {
      "type": "string",
      "description": "One short sentence explaining the decision."
    }
  },
  "required": ["act", "reason"],
  "propertyOrdering": ["act", "reason"]
}"#;

#[derive(Debug, Deserialize)]
struct FilterJudgment {
    act: bool,
    reason: String,
}

/// Process a newly fetched inbox message (non-blocking wrapper).
pub fn process_inbound_email(
    state: Arc<LlmAssistantState>,
    uid: u32,
    from: String,
    subject: String,
    raw: Vec<u8>,
) {
    tokio::spawn(async move {
        if let Err(e) = process_inbound_email_inner(&state, uid, &from, &subject, &raw).await {
            tracing::error!(target: LOG_TARGET, "email pipeline uid={uid}: {e:#}");
        }
    });
}

async fn process_inbound_email_inner(
    state: &LlmAssistantState,
    uid: u32,
    from: &str,
    subject: &str,
    raw: &[u8],
) -> anyhow::Result<()> {
    let parsed = match parse_rfc822(raw, from, subject) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(target: LOG_TARGET, uid, "MIME parse failed: {e:#}");
            return Ok(());
        }
    };

    let dedup_key = parsed.dedup_key(uid, from, subject);
    if is_already_processed(&state.db, &dedup_key).await? {
        tracing::warn!(
            target: LOG_TARGET,
            uid,
            message_id = %dedup_key,
            "email already processed; skipping"
        );
        return Ok(());
    }

    let prefs = load_preferences(&state.db).await?;
    let filter = prefs.email_filter.trim();
    if filter.is_empty() {
        tracing::warn!(target: LOG_TARGET, "email filter not configured; ignoring uid={uid}");
        return Ok(());
    }

    let judgment = match run_filter_judgment(state, filter, &parsed).await {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!(target: LOG_TARGET, uid, "filter LLM failed: {e:#}; skipping mail");
            return Ok(());
        }
    };
    tracing::warn!(
        target: LOG_TARGET,
        uid,
        from = %parsed.from_display,
        subject = %parsed.subject,
        act = judgment.act,
        reason = %judgment.reason,
        "email filter judgment"
    );
    if !judgment.act {
        return Ok(());
    }

    let owner_id = prefs.email_owner_user_id.filter(|id| *id > 0);
    let Some(owner_id) = owner_id else {
        tracing::error!(target: LOG_TARGET, "email owner not configured; skipping uid={uid}");
        return Ok(());
    };

    let owner = UserEntity::find_by_id(owner_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("session owner user {owner_id} not found"))?;

    let sender = reply_email_address(&parsed);
    let title = truncate_subject(&parsed.subject);
    let saved_vnodes = save_email_attachments(
        &state.db,
        state.email_automation.store.as_ref(),
        prefs.email_attachments_parent_id,
        uid,
        &parsed.attachments,
    )
    .await;

    let session_id = create_email_session(
        state,
        owner.id,
        &title,
        &sender,
        parsed.message_id.as_deref(),
        parsed.references.as_deref(),
    )
    .await?;

    mark_processed(&state.db, &dedup_key, uid, Some(session_id)).await?;

    let genai = state.genai_with_key().await?;
    let user_content = build_inbound_content(&genai, uid, &parsed, &saved_vnodes).await;

    tracing::warn!(
        target: LOG_TARGET,
        "email uid={uid} acting — session={session_id} reply_to={sender}"
    );

    // Register in live_turns so an open WebSocket can attach and stream events
    // (same path as handlers/ws.rs for UI-initiated turns).
    let (tx, _rx) = live_turn::new_turn_channel();
    let cancel = tokio_util::sync::CancellationToken::new();
    state
        .live_turns
        .insert(session_id, tx.clone(), cancel.clone());
    let store = Arc::clone(&state.email_automation.store);
    let tools = Arc::clone(&state.email_automation.tools);
    let rune_env = Arc::clone(&state.email_automation.rune_env);
    let result = run_stream_turn(
        state,
        store,
        tools,
        rune_env,
        session_id,
        user_content,
        tx,
        cancel,
        None,
    )
    .await;
    state.live_turns.remove(session_id);
    result?;

    Ok(())
}

async fn is_already_processed(
    db: &sea_orm::DatabaseConnection,
    message_id: &str,
) -> anyhow::Result<bool> {
    let found = ProcessedEmailEntity::find()
        .filter(processed_email::Column::MessageId.eq(message_id))
        .one(db)
        .await?;
    Ok(found.is_some())
}

async fn mark_processed(
    db: &sea_orm::DatabaseConnection,
    message_id: &str,
    uid: u32,
    session_id: Option<i64>,
) -> anyhow::Result<()> {
    let model = processed_email::ActiveModel {
        id: Default::default(),
        message_id: Set(message_id.to_string()),
        imap_uid: Set(Some(uid as i32)),
        session_id: Set(session_id),
        processed_at: Set(Utc::now()),
    };
    model.insert(db).await?;
    Ok(())
}

async fn run_filter_judgment(
    state: &LlmAssistantState,
    filter: &str,
    parsed: &ParsedEmail,
) -> anyhow::Result<FilterJudgment> {
    let system = format!(
        "You triage inbound email for an assistant inbox.\n\n\
         Filter instructions:\n{filter}\n\n\
         Decide whether the assistant should act on this message. \
         Output must match the response schema (act + reason)."
    );
    let attachment_meta = attachment_metadata_lines(&parsed.attachments);
    let mut user = format!(
        "From: {}\nSubject: {}\n",
        parsed.from_display, parsed.subject
    );
    if let Some(id) = parsed.message_id.as_deref() {
        user.push_str(&format!("Message-ID: {id}\n"));
    }
    if !attachment_meta.is_empty() {
        user.push_str("\nAttachments:\n");
        user.push_str(&attachment_meta);
        user.push('\n');
    }
    user.push_str("\nBody:\n");
    user.push_str(&parsed.body_text.chars().take(8000).collect::<String>());

    let genai = state.genai_with_key().await?;
    let raw = genai
        .generate_json(
            &system,
            &user,
            serde_json::from_str(FILTER_SCHEMA)?,
            FILTER_MAX_OUTPUT_TOKENS,
        )
        .await
        .map_err(|e| anyhow::anyhow!("filter LLM: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| anyhow::anyhow!("filter JSON: {e}: {raw}"))
}

async fn create_email_session(
    state: &LlmAssistantState,
    user_id: i64,
    title: &str,
    reply_email: &str,
    email_message_id: Option<&str>,
    email_references: Option<&str>,
) -> anyhow::Result<i64> {
    let now = Utc::now();
    let reply = if reply_email.is_empty() {
        None
    } else {
        Some(reply_email.to_string())
    };
    let model = session::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(title.to_string()),
        user_id: Set(user_id),
        reply_email: Set(reply),
        email_message_id: Set(email_message_id.map(str::to_string)),
        email_references: Set(email_references.map(str::to_string)),
    };
    let saved = model.insert(&state.db).await?;
    Ok(saved.id)
}

/// Build the user turn for an inbound email.
///
/// Attachments are uploaded once to the Gemini Files API and referenced via
/// `file_data` (URI), so later tool rounds do not re-embed base64 payloads.
/// VNode save notes are kept so Rune `read_file` remains available for durable access.
async fn build_inbound_content(
    genai: &GenaiClient,
    uid: u32,
    parsed: &ParsedEmail,
    saved_vnodes: &[(String, i64)],
) -> Content {
    let mut parts = vec![Part {
        text: Some(inbound_header_text(uid, parsed)),
        ..Default::default()
    }];

    for att in &parsed.attachments {
        let vnode_id = saved_vnodes
            .iter()
            .find(|(name, _)| name == &att.filename)
            .map(|(_, id)| *id);
        match upload_attachment_part(genai, &att.filename, &att.mime_type, &att.bytes).await {
            Ok(mut part) => {
                part.vnode_id = vnode_id;
                parts.push(part);
            }
            Err(e) => {
                tracing::error!(
                    target: LOG_TARGET,
                    filename = %att.filename,
                    error = %e,
                    "Files API upload failed; attachment available via Rune read_file if saved"
                );
            }
        }
    }

    if let Some(note) = saved_attachments_note(saved_vnodes) {
        parts.push(Part {
            text: Some(note),
            ..Default::default()
        });
    }

    Content {
        role: Role::User,
        parts,
    }
}

fn inbound_header_text(uid: u32, parsed: &ParsedEmail) -> String {
    let mut header = format!(
        "Incoming email (UID {uid})\n\nFrom: {}\nSubject: {}",
        parsed.from_display, parsed.subject
    );
    if let Some(id) = parsed.message_id.as_deref() {
        header.push_str(&format!("\nMessage-ID: {id}"));
    }
    if !parsed.body_text.trim().is_empty() {
        header.push_str("\n\n");
        header.push_str(parsed.body_text.trim());
    }
    header
}

fn saved_attachments_note(saved_vnodes: &[(String, i64)]) -> Option<String> {
    if saved_vnodes.is_empty() {
        return None;
    }
    let note = saved_vnodes
        .iter()
        .map(|(name, id)| format!("{name} (vnode {id})"))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("Saved attachments for Rune read_file: {note}"))
}

fn truncate_subject(subject: &str) -> String {
    const MAX: usize = 120;
    let trimmed = subject.trim();
    if trimmed.chars().count() <= MAX {
        return trimmed.to_string();
    }
    trimmed.chars().take(MAX).collect::<String>() + "…"
}

fn reply_email_address(parsed: &ParsedEmail) -> String {
    extract_reply_address(&parsed.reply_to)
}

/// Pull a single reply address from a formatted From/Reply-To header.
fn extract_reply_address(from: &str) -> String {
    let trimmed = from.trim();
    if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed.find('>') {
            return trimmed[start + 1..end].trim().to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_reply_address_angle_brackets() {
        assert_eq!(
            extract_reply_address("Alice <alice@example.com>"),
            "alice@example.com"
        );
    }

    #[test]
    fn truncate_subject_long() {
        let long = "x".repeat(200);
        assert_eq!(truncate_subject(&long).chars().count(), 121);
    }

    #[test]
    fn inbound_header_includes_body_and_message_id() {
        let parsed = ParsedEmail {
            message_id: Some("<m@test>".into()),
            reply_to: "a@b".into(),
            references: None,
            subject: "Hi".into(),
            body_text: "Hello".into(),
            attachments: vec![],
            from_display: "Alice".into(),
            date: None,
        };
        let header = inbound_header_text(1, &parsed);
        assert!(header.contains("UID 1"));
        assert!(header.contains("Message-ID: <m@test>"));
        assert!(header.contains("Hello"));
        assert!(saved_attachments_note(&[]).is_none());
    }

    #[test]
    fn saved_attachments_note_lists_vnodes() {
        let note = saved_attachments_note(&[("doc.txt".into(), 9)]).expect("note");
        assert!(note.contains("doc.txt (vnode 9)"));
        assert!(note.contains("read_file"));
    }
}
