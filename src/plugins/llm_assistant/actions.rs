//! Assistant chat actions — streaming multi-round tool loop (Phase 4).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::{
    llm_tools::{LlmToolsCapability, ToolCtx},
    plugins::filesystem::storage::DynFilestore,
    rune_env::RuneEnvCapability,
};

use super::{
    config::{ASSISTANT_TOOL_ROUNDS, CHAT_MAX_OUTPUT_TOKENS},
    content::{
        PersistError, ZWSP, load_session_contents, save_content, strip_display_name_from_contents,
    },
    entities::session::{self, Entity as SessionEntity},
    genai::{Content, FunctionResponse, GenaiError, Part, ROLE_MODEL, ROLE_USER},
    state::LlmAssistantState,
};

#[derive(Debug, Error)]
pub enum ActionError {
    #[error(transparent)]
    Persist(#[from] PersistError),
    #[error(transparent)]
    Genai(#[from] GenaiError),
    #[error("database: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("{0}")]
    Other(String),
}

/// Events emitted during a streaming turn (WS layer builds OOB HTML).
#[derive(Debug, Clone)]
pub enum StreamEvent {
    UserSaved {
        session_id: i64,
        user: Content,
    },
    /// Live stream chunks (UI no longer shows a stream panel).
    Partial(Content),
    /// Model turn that includes function calls (args shown in transcript).
    ToolCall(Content),
    Tool(Content),
    Final(Content),
}

/// Split history vs last user turn.
pub fn split_last_user_content(
    contents: &[Content],
) -> Result<(Vec<Content>, Content), ActionError> {
    let last = contents
        .last()
        .ok_or_else(|| ActionError::Other("empty session".into()))?;
    if !last.role.eq_ignore_ascii_case(ROLE_USER) {
        return Err(ActionError::Other(format!(
            "last message must be user (got {:?})",
            last.role
        )));
    }
    if last.parts.is_empty() {
        return Err(ActionError::Other("last user message has no parts".into()));
    }
    let history = contents[..contents.len() - 1].to_vec();
    Ok((history, last.clone()))
}

pub fn content_has_function_call(content: &Content) -> bool {
    content.parts.iter().any(|p| p.function_call.is_some())
}

pub fn content_has_tool_response_parts(content: &Content) -> bool {
    content
        .parts
        .iter()
        .any(|p| p.function_response.is_some() || p.tool_response.is_some())
}

async fn bump_session(
    db: &sea_orm::DatabaseConnection,
    session_id: i64,
) -> Result<(), ActionError> {
    if let Some(sess) = SessionEntity::find_by_id(session_id).one(db).await? {
        let mut am: session::ActiveModel = sess.into();
        am.updated_at = Set(Some(chrono::Utc::now()));
        am.update(db).await?;
    }
    Ok(())
}

/// Save user text → generateContent → save model reply (legacy / tests; no tools).
pub async fn run_one_turn(
    state: &LlmAssistantState,
    session_id: i64,
    message: &str,
) -> Result<(), ActionError> {
    let message = message.trim();
    if message.is_empty() {
        return Err(ActionError::Other("empty message".into()));
    }

    let user = Content::text(ROLE_USER, message);
    save_content(&state.db, session_id, &user).await?;
    bump_session(&state.db, session_id).await?;

    let mut contents = load_session_contents(&state.db, session_id).await?;
    strip_display_name_from_contents(&mut contents);
    let (history, last_user) = split_last_user_content(&contents)?;
    let mut for_api = history;
    for_api.push(last_user);

    let genai = state
        .genai_with_key()
        .await
        .map_err(|e| ActionError::Other(e.to_string()))?;
    let mut model = genai
        .generate_content(for_api, CHAT_MAX_OUTPUT_TOKENS, &[])
        .await?;
    if model.role.is_empty() {
        model.role = ROLE_MODEL.to_string();
    }
    if model.parts.is_empty() {
        return Err(ActionError::Other("empty model response".into()));
    }
    save_content(&state.db, session_id, &model).await?;
    bump_session(&state.db, session_id).await?;

    Ok(())
}

async fn run_tool_round(
    tools: &LlmToolsCapability,
    ctx: &ToolCtx<'_>,
    name: &str,
    args: Option<&Value>,
) -> Value {
    let args = args.cloned().unwrap_or(Value::Object(Default::default()));
    match tools.get(name) {
        None => json!({ "error": "unknown tool" }),
        Some(tool) => match tool.run(ctx, args).await {
            Ok(v) => v,
            Err(e) => json!({ "error": e }),
        },
    }
}

/// Streaming turn with multi-round function calling.
pub async fn run_stream_turn(
    state: &LlmAssistantState,
    store: Arc<DynFilestore>,
    tools: Arc<LlmToolsCapability>,
    rune_env: Arc<RuneEnvCapability>,
    session_id: i64,
    user: Content,
    tx: mpsc::UnboundedSender<StreamEvent>,
) -> Result<(), ActionError> {
    if user.parts.is_empty() {
        return Err(ActionError::Other("message is empty".into()));
    }

    save_content(&state.db, session_id, &user).await?;
    bump_session(&state.db, session_id).await?;
    let _ = tx.send(StreamEvent::UserSaved {
        session_id,
        user: user.clone(),
    });

    let decls = tools.declarations();
    let max_rounds = ASSISTANT_TOOL_ROUNDS.max(1);

    for _round in 0..max_rounds {
        let mut contents = load_session_contents(&state.db, session_id).await?;
        strip_display_name_from_contents(&mut contents);
        let (history, last_user) = split_last_user_content(&contents)?;
        let mut for_api = history;
        for_api.push(last_user);

        let (partial_tx, mut partial_rx) = mpsc::unbounded_channel::<Content>();
        let genai = state
            .genai_with_key()
            .await
            .map_err(|e| ActionError::Other(e.to_string()))?;
        let decls_clone = decls.clone();
        let join = tokio::spawn(async move {
            genai
                .stream_generate_content(for_api, CHAT_MAX_OUTPUT_TOKENS, &decls_clone, |merged| {
                    let _ = partial_tx.send(merged.clone());
                })
                .await
        });

        while let Some(partial) = partial_rx.recv().await {
            let _ = tx.send(StreamEvent::Partial(partial));
        }

        let mut model = join
            .await
            .map_err(|e| ActionError::Other(format!("stream task: {e}")))??;
        if model.role.is_empty() {
            model.role = ROLE_MODEL.to_string();
        }
        if model.parts.is_empty() {
            return Err(ActionError::Other("empty model response".into()));
        }

        if content_has_function_call(&model) {
            save_content(&state.db, session_id, &model).await?;
            bump_session(&state.db, session_id).await?;
            // Show the tool call (with args) in the transcript before the response.
            let _ = tx.send(StreamEvent::ToolCall(model.clone()));

            let tool_ctx = ToolCtx {
                db: &state.db,
                store: Arc::clone(&store),
                cse_api_key: &state.config.cse_api_key,
                cse_cx: &state.config.cse_cx,
                rune_env: &rune_env,
            };

            let mut resp_parts = Vec::new();
            for part in &model.parts {
                let Some(fc) = &part.function_call else {
                    continue;
                };
                let res = run_tool_round(&tools, &tool_ctx, &fc.name, fc.args.as_ref()).await;
                resp_parts.push(Part {
                    function_response: Some(FunctionResponse {
                        function_response_id: fc.id.clone(),
                        name: fc.name.clone(),
                        response: Some(res),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
            if resp_parts.is_empty() {
                return Err(ActionError::Other(
                    "model message had no usable function calls".into(),
                ));
            }

            let user_tool = Content {
                role: ROLE_USER.to_string(),
                parts: resp_parts,
            };
            save_content(&state.db, session_id, &user_tool).await?;
            bump_session(&state.db, session_id).await?;
            let _ = tx.send(StreamEvent::Tool(user_tool));
            continue;
        }

        save_content(&state.db, session_id, &model).await?;
        bump_session(&state.db, session_id).await?;
        let _ = tx.send(StreamEvent::Final(model));
        return Ok(());
    }

    Err(ActionError::Other("tool round limit exceeded".into()))
}

/// Build HTML transcript from loaded contents (hide ZWSP-only parts).
pub fn transcript_html(contents: &[Content]) -> String {
    use crate::plugins::llm_assistant::ws::html::{
        assistant_bubble_html, tool_bubble_html, user_bubble_html,
    };

    let mut out = String::new();
    for c in contents {
        if content_has_tool_response_parts(c) {
            out.push_str(&tool_bubble_html(c));
            continue;
        }

        let role = c.role.to_lowercase();
        let kind = if role == ROLE_MODEL || role == "assistant" {
            "assistant"
        } else {
            "user"
        };
        let has_visible = c.parts.iter().any(|p| {
            p.text.as_ref().is_some_and(|t| t != ZWSP && !t.is_empty())
                || p.inline_data.is_some()
                || p.function_call.is_some()
        });
        if !has_visible {
            continue;
        }
        if kind == "assistant" {
            out.push_str(&assistant_bubble_html(c));
        } else {
            out.push_str(&user_bubble_html(c));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genai::FunctionCall;

    #[test]
    fn detects_function_call() {
        let c = Content {
            role: ROLE_MODEL.into(),
            parts: vec![Part {
                function_call: Some(FunctionCall {
                    name: "list_skills".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }],
        };
        assert!(content_has_function_call(&c));
        assert!(!content_has_tool_response_parts(&c));
    }

    #[test]
    fn transcript_renders_function_call_args() {
        let contents = vec![Content {
            role: ROLE_MODEL.into(),
            parts: vec![Part {
                function_call: Some(FunctionCall {
                    name: "read_file".into(),
                    args: Some(serde_json::json!({ "path": "/tmp/foo.txt" })),
                    ..Default::default()
                }),
                ..Default::default()
            }],
        }];
        let html = transcript_html(&contents);
        assert!(html.contains("Function call: read_file"));
        assert!(html.contains("Arguments"));
        assert!(html.contains("/tmp/foo.txt"));
    }
}
