//! Assistant chat actions — streaming multi-round tool loop (Phase 4).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};

use crate::{
    llm_tools::{LlmToolsCapability, ToolCtx},
    plugins::filesystem::storage::DynFilestore,
    rune_env::RuneEnvCapability,
};

use super::{
    config::{ASSISTANT_TOOL_ROUNDS, CHAT_MAX_OUTPUT_TOKENS},
    content::{PersistError, ZWSP, load_session_contents, save_content},
    entities::session::{self, Entity as SessionEntity},
    genai::{Content, FunctionResponse, GenaiError, Part, Role},
    live_turn,
    state::LlmAssistantState,
};

pub use super::live_turn::StreamEvent;

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

/// Split history vs last user turn.
pub fn split_last_user_content(
    contents: &[Content],
) -> Result<(Vec<Content>, Content), ActionError> {
    let last = contents
        .last()
        .ok_or_else(|| ActionError::Other("empty session".into()))?;
    if last.role != Role::User {
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

/// Text (or attachment names) used to autogenerate a session title.
fn prompt_text_for_title(user: &Content) -> String {
    let texts: Vec<&str> = user
        .parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .map(str::trim)
        .filter(|t| !t.is_empty() && *t != ZWSP)
        .collect();
    if !texts.is_empty() {
        return texts.join(" ");
    }
    user.parts
        .iter()
        .map(|p| p.display_name.as_str())
        .filter(|n| !n.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

async fn maybe_title_from_first_prompt(
    db: &sea_orm::DatabaseConnection,
    session_id: i64,
    user: &Content,
) -> Result<Option<String>, ActionError> {
    let prompt = prompt_text_for_title(user);
    super::handlers::history::maybe_set_session_title_from_prompt(db, session_id, &prompt)
        .await
        .map_err(ActionError::Other)
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

    let user = Content::text(Role::User, message);
    save_content(&state.db, session_id, &user).await?;
    bump_session(&state.db, session_id).await?;
    let _ = maybe_title_from_first_prompt(&state.db, session_id, &user).await?;

    let contents = load_session_contents(&state.db, session_id).await?;
    let (history, last_user) = split_last_user_content(&contents)?;
    let mut for_api = history;
    for_api.push(last_user);

    let genai = state
        .genai_with_key()
        .await
        .map_err(|e| ActionError::Other(e.to_string()))?;
    let model = genai
        .generate_content(for_api, CHAT_MAX_OUTPUT_TOKENS, &[])
        .await?;
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
    tx: broadcast::Sender<StreamEvent>,
) -> Result<(), ActionError> {
    if user.parts.is_empty() {
        return Err(ActionError::Other("message is empty".into()));
    }

    save_content(&state.db, session_id, &user).await?;
    bump_session(&state.db, session_id).await?;
    let title = maybe_title_from_first_prompt(&state.db, session_id, &user).await?;
    live_turn::emit(
        &tx,
        StreamEvent::UserSaved {
            session_id,
            user: user.clone(),
            title,
        },
    );

    let decls = tools.declarations();
    let max_rounds = ASSISTANT_TOOL_ROUNDS.max(1);
    let prefs = super::preferences::load_preferences(&state.db).await?;
    let cse_api_key = prefs.cse_api_key;
    let cse_cx = prefs.cse_cx;

    for _round in 0..max_rounds {
        let contents = load_session_contents(&state.db, session_id).await?;
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
            live_turn::emit(&tx, StreamEvent::Partial(partial));
        }

        let model = join
            .await
            .map_err(|e| ActionError::Other(format!("stream task: {e}")))??;
        if model.parts.is_empty() {
            return Err(ActionError::Other("empty model response".into()));
        }

        if content_has_function_call(&model) {
            save_content(&state.db, session_id, &model).await?;
            bump_session(&state.db, session_id).await?;
            // Show the tool call (with args) in the transcript before the response.
            live_turn::emit(&tx, StreamEvent::ToolCall(model.clone()));

            let tool_ctx = ToolCtx {
                db: &state.db,
                store: Arc::clone(&store),
                cse_api_key: &cse_api_key,
                cse_cx: &cse_cx,
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
                role: Role::User,
                parts: resp_parts,
            };
            save_content(&state.db, session_id, &user_tool).await?;
            bump_session(&state.db, session_id).await?;
            live_turn::emit(&tx, StreamEvent::Tool(user_tool));
            continue;
        }

        save_content(&state.db, session_id, &model).await?;
        bump_session(&state.db, session_id).await?;
        live_turn::emit(&tx, StreamEvent::Final(model));
        return Ok(());
    }

    Err(ActionError::Other("tool round limit exceeded".into()))
}

/// Build HTML transcript from loaded contents (hide ZWSP-only parts).
pub fn transcript_html(contents: &[Content]) -> String {
    use crate::plugins::llm_assistant::ws::html::{
        assistant_bubble_html, tool_call_inner_html, tool_response_inner_html, user_bubble_html,
        working_group_html,
    };

    let mut out = String::new();
    let mut i = 0;
    while i < contents.len() {
        let c = &contents[i];

        // Coalesce consecutive tool call / tool response turns under one Tools Called dropdown.
        if content_has_function_call(c) || content_has_tool_response_parts(c) {
            let mut working = String::new();
            while i < contents.len() {
                let cur = &contents[i];
                if content_has_function_call(cur) {
                    working.push_str(&tool_call_inner_html(cur));
                    i += 1;
                } else if content_has_tool_response_parts(cur) {
                    working.push_str(&tool_response_inner_html(cur));
                    i += 1;
                } else {
                    break;
                }
            }
            out.push_str(&working_group_html(&working));
            continue;
        }

        let kind = if c.role == Role::Model {
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
            i += 1;
            continue;
        }
        if kind == "assistant" {
            out.push_str(&assistant_bubble_html(c));
        } else {
            out.push_str(&user_bubble_html(c));
        }
        i += 1;
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
            role: Role::Model,
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
            role: Role::Model,
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
        assert!(html.contains("Tools Called"));
        assert!(html.contains("Function call: read_file"));
        assert!(html.contains("Arguments"));
        assert!(html.contains("/tmp/foo.txt"));
    }

    #[test]
    fn transcript_groups_tool_call_and_response() {
        let contents = vec![
            Content {
                role: Role::User,
                parts: vec![Part {
                    text: Some("hi".into()),
                    ..Default::default()
                }],
            },
            Content {
                role: Role::Model,
                parts: vec![Part {
                    function_call: Some(FunctionCall {
                        name: "list_skills".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
            },
            Content {
                role: Role::User,
                parts: vec![Part {
                    function_response: Some(FunctionResponse {
                        name: "list_skills".into(),
                        response: Some(serde_json::json!({ "ok": true })),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
            },
            Content {
                role: Role::Model,
                parts: vec![Part {
                    text: Some("done".into()),
                    ..Default::default()
                }],
            },
        ];
        let html = transcript_html(&contents);
        assert_eq!(html.matches("Tools Called").count(), 1);
        assert!(!html.contains("Tool Execution"));
        assert!(html.contains("Function call: list_skills"));
        assert!(html.contains("Function response: list_skills"));
        assert!(html.contains("done"));
    }
}
