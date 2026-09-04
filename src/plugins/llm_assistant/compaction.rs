//! Chat compaction: summarize a session epoch and fence older messages from the API.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use thiserror::Error;

use crate::genai::{Content, GenaiError, Role};

use super::{
    config::COMPACTION_MAX_OUTPUT_TOKENS,
    content::{PersistError, SessionTurn, load_session_turns},
    entities::session_compaction::{self, Entity as CompactionEntity},
    preferences::resolved_compactor_model,
    state::LlmAssistantState,
};

const COMPACTOR_SYSTEM_PROMPT: &str = "\
You compact a chat transcript into a concise summary so another assistant can continue the conversation.

Preserve: user goals, decisions, constraints, names/IDs, unresolved tasks, and important tool outcomes.
Omit: chit-chat, duplicated tool payloads, and the fact that this is a summary unless essential.
Write factual notes. Do not address the user. Do not mention tools you cannot call.";

const SUMMARY_USER_PREFIX: &str = "The following is a summary of the conversation so far. \
Continue from this summary. Do not mention the summary unless asked.\n\n";
const SUMMARY_ACK: &str = "Understood. I will continue from that summary.";

const TOOL_PAYLOAD_CHAR_LIMIT: usize = 2_000;

#[derive(Debug, Error)]
pub enum CompactionError {
    #[error(transparent)]
    Persist(#[from] PersistError),
    #[error(transparent)]
    Genai(#[from] GenaiError),
    #[error("database: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("{0}")]
    Other(String),
}

/// Fence: messages with `id <= through_message_id` are represented by `summary` for the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionFence {
    pub through_message_id: i64,
    pub summary: String,
}

impl From<&session_compaction::Model> for CompactionFence {
    fn from(row: &session_compaction::Model) -> Self {
        Self {
            through_message_id: row.through_message_id,
            summary: row.summary.clone(),
        }
    }
}

pub fn latest_fence(fences: &[CompactionFence]) -> Option<&CompactionFence> {
    fences.iter().max_by_key(|f| f.through_message_id)
}

/// Gemini contents for the next generate: summary pair + messages after the last fence.
pub fn contents_for_api(turns: &[SessionTurn], latest: Option<&CompactionFence>) -> Vec<Content> {
    let mut out = Vec::new();
    if let Some(fence) = latest {
        out.push(summary_user_content(&fence.summary));
        out.push(summary_ack_content());
        out.extend(
            turns
                .iter()
                .filter(|t| t.id > fence.through_message_id)
                .map(|t| t.content.clone()),
        );
    } else {
        out.extend(turns.iter().map(|t| t.content.clone()));
    }
    out
}

pub fn summary_user_content(summary: &str) -> Content {
    Content::text(Role::User, format!("{SUMMARY_USER_PREFIX}{summary}"))
}

pub fn summary_ack_content() -> Content {
    Content::text(Role::Model, SUMMARY_ACK)
}

pub async fn load_session_fences(
    db: &DatabaseConnection,
    session_id: i64,
) -> Result<Vec<CompactionFence>, sea_orm::DbErr> {
    let rows = CompactionEntity::find()
        .filter(session_compaction::Column::LlmAssistantSessionId.eq(session_id))
        .order_by_asc(session_compaction::Column::Id)
        .all(db)
        .await?;
    Ok(rows.iter().map(CompactionFence::from).collect())
}

/// Summarize the current API epoch and persist a fence through the last message.
///
/// Returns `None` when there is nothing new to compact.
pub async fn compact_session(
    state: &LlmAssistantState,
    session_id: i64,
) -> Result<Option<String>, CompactionError> {
    let turns = load_session_turns(&state.db, session_id).await?;
    let Some(last) = turns.last() else {
        return Ok(None);
    };
    let fences = load_session_fences(&state.db, session_id).await?;
    let latest = latest_fence(&fences);
    let epoch: Vec<&SessionTurn> = match latest {
        Some(f) => turns
            .iter()
            .filter(|t| t.id > f.through_message_id)
            .collect(),
        None => turns.iter().collect(),
    };
    if epoch.is_empty() {
        return Ok(None);
    }

    let user_prompt = epoch_text(latest.map(|f| f.summary.as_str()), &epoch);
    if user_prompt.trim().is_empty() {
        return Ok(None);
    }

    let genai = state
        .genai_with_key()
        .await
        .map_err(|e| CompactionError::Other(e.to_string()))?;
    let model = resolved_compactor_model(&state.db)
        .await
        .map_err(CompactionError::Db)?;
    let genai = genai.with_model(model);
    let summary = genai
        .generate_text_with_tokens(
            COMPACTOR_SYSTEM_PROMPT,
            &user_prompt,
            COMPACTION_MAX_OUTPUT_TOKENS,
        )
        .await?
        .trim()
        .to_string();
    if summary.is_empty() {
        return Err(CompactionError::Other("empty compaction summary".into()));
    }

    let now = Utc::now();
    session_compaction::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        llm_assistant_session_id: Set(session_id),
        through_message_id: Set(last.id),
        summary: Set(summary.clone()),
    }
    .insert(&state.db)
    .await?;

    Ok(Some(summary))
}

fn epoch_text(previous_summary: Option<&str>, turns: &[&SessionTurn]) -> String {
    let mut out = String::new();
    if let Some(summary) = previous_summary.filter(|s| !s.trim().is_empty()) {
        out.push_str("Previous conversation summary:\n");
        out.push_str(summary);
        out.push_str("\n\n");
    }
    out.push_str("Transcript:\n");
    for turn in turns {
        append_turn(&mut out, turn);
    }
    out
}

fn append_turn(out: &mut String, turn: &SessionTurn) {
    let role = match turn.content.role {
        Role::User => "User",
        Role::Model => "Assistant",
    };
    out.push_str(&format!("\n{role}:\n"));
    for part in &turn.content.parts {
        if part.thought {
            continue;
        }
        if let Some(text) = part.text.as_deref().map(str::trim) {
            if !text.is_empty() && text != super::content::ZWSP {
                out.push_str(text);
                out.push('\n');
            }
        }
        if part.inline_data.is_some() || part.file_data.is_some() {
            let name = if part.display_name.is_empty() {
                "attachment"
            } else {
                part.display_name.as_str()
            };
            out.push_str(&format!("[attachment: {name}]\n"));
        }
        if let Some(fc) = &part.function_call {
            out.push_str(&format!("Function call: {}\n", fc.name));
            if let Some(args) = &fc.args {
                out.push_str(&truncate_json(args));
                out.push('\n');
            }
        }
        if let Some(fr) = &part.function_response {
            out.push_str(&format!("Function response: {}\n", fr.name));
            if let Some(resp) = &fr.response {
                out.push_str(&truncate_json(resp));
                out.push('\n');
            }
        }
        if let Some(tr) = &part.tool_response {
            out.push_str("Tool response:\n");
            if let Some(resp) = &tr.response {
                out.push_str(&truncate_json(resp));
                out.push('\n');
            }
        }
    }
}

fn truncate_json(v: &serde_json::Value) -> String {
    let raw = serde_json::to_string(v).unwrap_or_default();
    if raw.len() <= TOOL_PAYLOAD_CHAR_LIMIT {
        return raw;
    }
    let mut cut = TOOL_PAYLOAD_CHAR_LIMIT;
    while cut > 0 && !raw.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}…", &raw[..cut])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genai::{FunctionCall, FunctionResponse, Part};

    fn turn(id: i64, role: Role, text: &str) -> SessionTurn {
        SessionTurn {
            id,
            content: Content::text(role, text),
        }
    }

    #[test]
    fn api_contents_without_fence_keeps_all() {
        let turns = vec![turn(1, Role::User, "hi"), turn(2, Role::Model, "hello")];
        let api = contents_for_api(&turns, None);
        assert_eq!(api.len(), 2);
        assert_eq!(api[0].parts[0].text.as_deref(), Some("hi"));
        assert_eq!(api[1].parts[0].text.as_deref(), Some("hello"));
    }

    #[test]
    fn api_contents_drops_messages_through_fence() {
        let turns = vec![
            turn(1, Role::User, "old"),
            turn(2, Role::Model, "old reply"),
            turn(3, Role::User, "new"),
        ];
        let fence = CompactionFence {
            through_message_id: 2,
            summary: "Prior chat about old.".into(),
        };
        let api = contents_for_api(&turns, Some(&fence));
        assert_eq!(api.len(), 3);
        assert!(
            api[0].parts[0]
                .text
                .as_deref()
                .is_some_and(|t| t.contains("Prior chat about old."))
        );
        assert_eq!(api[0].role, Role::User);
        assert_eq!(api[1].role, Role::Model);
        assert_eq!(api[2].parts[0].text.as_deref(), Some("new"));
    }

    #[test]
    fn second_compaction_uses_latest_fence_only() {
        let turns = vec![
            turn(1, Role::User, "a"),
            turn(2, Role::Model, "b"),
            turn(3, Role::User, "c"),
            turn(4, Role::Model, "d"),
            turn(5, Role::User, "e"),
        ];
        let fences = [
            CompactionFence {
                through_message_id: 2,
                summary: "first".into(),
            },
            CompactionFence {
                through_message_id: 4,
                summary: "second".into(),
            },
        ];
        let api = contents_for_api(&turns, latest_fence(&fences));
        assert_eq!(api.len(), 3);
        assert!(
            api[0].parts[0]
                .text
                .as_deref()
                .is_some_and(|t| t.contains("second") && !t.contains("first"))
        );
        assert_eq!(api[2].parts[0].text.as_deref(), Some("e"));
    }

    #[test]
    fn epoch_text_includes_prior_summary_and_skips_thoughts() {
        let prior = CompactionFence {
            through_message_id: 1,
            summary: "Earlier work.".into(),
        };
        let turns = [
            SessionTurn {
                id: 2,
                content: Content {
                    role: Role::Model,
                    parts: vec![
                        Part {
                            thought: true,
                            text: Some("secret".into()),
                            ..Default::default()
                        },
                        Part {
                            text: Some("visible".into()),
                            ..Default::default()
                        },
                        Part {
                            function_call: Some(FunctionCall {
                                name: "read_file".into(),
                                args: Some(serde_json::json!({ "path": "/tmp/x" })),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    ],
                },
            },
            SessionTurn {
                id: 3,
                content: Content {
                    role: Role::User,
                    parts: vec![Part {
                        function_response: Some(FunctionResponse {
                            name: "read_file".into(),
                            response: Some(serde_json::json!({ "ok": true })),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                },
            },
        ];
        let refs: Vec<&SessionTurn> = turns.iter().collect();
        let text = epoch_text(Some(prior.summary.as_str()), &refs);
        assert!(text.contains("Earlier work."));
        assert!(text.contains("visible"));
        assert!(!text.contains("secret"));
        assert!(text.contains("Function call: read_file"));
        assert!(text.contains("Function response: read_file"));
    }
}
