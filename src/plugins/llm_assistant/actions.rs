//! Assistant chat actions — streaming multi-round tool loop (Phase 4).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::{
    genai::util::content_answer_text,
    llm_tools::{LlmToolsCapability, ToolCtx},
    plugins::filesystem::storage::DynFilestore,
    rune_env::RuneEnvCapability,
};

use super::{
    config::{ASSISTANT_TOOL_ROUNDS, CHAT_MAX_OUTPUT_TOKENS},
    content::{
        PersistError, ZWSP, elide_attachment_parts_for_api, load_session_contents, save_content,
    },
    email_send,
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
#[allow(clippy::too_many_arguments)]
pub async fn run_stream_turn(
    state: &LlmAssistantState,
    store: Arc<DynFilestore>,
    tools: Arc<LlmToolsCapability>,
    rune_env: Arc<RuneEnvCapability>,
    session_id: i64,
    user: Content,
    tx: broadcast::Sender<StreamEvent>,
    cancel: CancellationToken,
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

    // Keep an in-memory transcript for this turn so follow-up rounds do not
    // re-read attachment blobs from the DB. Attachments stay in DB/UI history.
    let mut contents = load_session_contents(&state.db, session_id).await?;
    let genai = state
        .genai_with_key()
        .await
        .map_err(|e| ActionError::Other(e.to_string()))?;

    let mut last_partial: Option<Content> = None;
    let mut unanswered_tool_calls: Option<Content> = None;
    let mut answered_tool_parts: Vec<Part> = Vec::new();

    for round in 0..max_rounds {
        if cancel.is_cancelled() {
            return finish_stopped(
                state,
                session_id,
                &tx,
                last_partial.take(),
                unanswered_tool_calls.take(),
                answered_tool_parts,
            )
            .await;
        }

        let (history, last_user) = split_last_user_content(&contents)?;
        let mut for_api = history;
        for_api.push(last_user);
        // First generate sees attachments; later tool rounds elide them so Gemini
        // does not re-process PDFs/images on every function-call hop.
        if round > 0 {
            elide_attachment_parts_for_api(&mut for_api);
        }

        let (partial_tx, mut partial_rx) = mpsc::unbounded_channel::<Content>();
        let genai = genai.clone();
        let decls_clone = decls.clone();
        let join = tokio::spawn(async move {
            genai
                .stream_generate_content(for_api, CHAT_MAX_OUTPUT_TOKENS, &decls_clone, |merged| {
                    let _ = partial_tx.send(merged.clone());
                })
                .await
        });

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    join.abort();
                    return finish_stopped(
                        state,
                        session_id,
                        &tx,
                        last_partial.take(),
                        unanswered_tool_calls.take(),
                        answered_tool_parts,
                    )
                    .await;
                }
                partial = partial_rx.recv() => {
                    match partial {
                        Some(p) => {
                            last_partial = Some(p.clone());
                            live_turn::emit(&tx, StreamEvent::Partial(p));
                        }
                        None => break,
                    }
                }
            }
        }

        let model = match join.await {
            Ok(Ok(m)) => m,
            Ok(Err(e)) => return Err(e.into()),
            Err(e) if e.is_cancelled() => {
                return finish_stopped(
                    state,
                    session_id,
                    &tx,
                    last_partial.take(),
                    unanswered_tool_calls.take(),
                    answered_tool_parts,
                )
                .await;
            }
            Err(e) => return Err(ActionError::Other(format!("stream task: {e}"))),
        };
        last_partial = None;
        if model.parts.is_empty() {
            return Err(ActionError::Other("empty model response".into()));
        }

        if content_has_function_call(&model) {
            save_content(&state.db, session_id, &model).await?;
            bump_session(&state.db, session_id).await?;
            // Show the tool call (with args) in the transcript before the response.
            live_turn::emit(&tx, StreamEvent::ToolCall(model.clone()));
            unanswered_tool_calls = Some(model.clone());
            answered_tool_parts.clear();

            let tool_ctx = ToolCtx {
                db: &state.db,
                store: Arc::clone(&store),
                cse_api_key: &cse_api_key,
                cse_cx: &cse_cx,
                rune_env: &rune_env,
                session_id: Some(session_id),
            };

            for part in &model.parts {
                let Some(fc) = &part.function_call else {
                    continue;
                };
                let res = tokio::select! {
                    _ = cancel.cancelled() => {
                        return finish_stopped(
                            state,
                            session_id,
                            &tx,
                            None,
                            unanswered_tool_calls.take(),
                            answered_tool_parts,
                        )
                        .await;
                    }
                    res = run_tool_round(&tools, &tool_ctx, &fc.name, fc.args.as_ref()) => res,
                };
                answered_tool_parts.push(Part {
                    function_response: Some(FunctionResponse {
                        function_response_id: fc.id.clone(),
                        name: fc.name.clone(),
                        response: Some(res),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
            if answered_tool_parts.is_empty() {
                return Err(ActionError::Other(
                    "model message had no usable function calls".into(),
                ));
            }

            let user_tool = Content {
                role: Role::User,
                parts: std::mem::take(&mut answered_tool_parts),
            };
            save_content(&state.db, session_id, &user_tool).await?;
            bump_session(&state.db, session_id).await?;
            live_turn::emit(&tx, StreamEvent::Tool(user_tool.clone()));
            contents.push(model);
            contents.push(user_tool);
            unanswered_tool_calls = None;
            continue;
        }

        save_content(&state.db, session_id, &model).await?;
        bump_session(&state.db, session_id).await?;
        live_turn::emit(&tx, StreamEvent::Final(model.clone()));
        spawn_reply_email_if_needed(state, session_id, &model).await;
        return Ok(());
    }

    Err(ActionError::Other("tool round limit exceeded".into()))
}

/// Visible (non-thought) text from a cancelled stream, dropping incomplete function calls.
fn model_text_from_partial(partial: &Content) -> Option<Content> {
    let parts: Vec<Part> = partial
        .parts
        .iter()
        .filter(|p| !p.thought)
        .filter_map(|p| {
            let t = p.text.as_deref()?.trim();
            if t.is_empty() || t == ZWSP {
                None
            } else {
                Some(Part {
                    text: Some(t.to_string()),
                    ..Default::default()
                })
            }
        })
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(Content {
            role: Role::Model,
            parts,
        })
    }
}

fn cancelled_fn_response(fc: &super::genai::FunctionCall) -> Part {
    Part {
        function_response: Some(FunctionResponse {
            function_response_id: fc.id.clone(),
            name: fc.name.clone(),
            response: Some(json!({ "error": "cancelled" })),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Fill in cancelled responses for function calls that never completed.
fn complete_cancelled_tool_parts(model: &Content, already: Vec<Part>) -> Vec<Part> {
    let answered: std::collections::HashSet<String> = already
        .iter()
        .filter_map(|p| {
            let fr = p.function_response.as_ref()?;
            Some(if !fr.function_response_id.is_empty() {
                fr.function_response_id.clone()
            } else {
                fr.name.clone()
            })
        })
        .collect();
    let mut parts = already;
    for part in &model.parts {
        let Some(fc) = &part.function_call else {
            continue;
        };
        let key = if !fc.id.is_empty() {
            fc.id.clone()
        } else {
            fc.name.clone()
        };
        if answered.contains(&key) {
            continue;
        }
        parts.push(cancelled_fn_response(fc));
    }
    parts
}

async fn finish_stopped(
    state: &LlmAssistantState,
    session_id: i64,
    tx: &broadcast::Sender<StreamEvent>,
    last_partial: Option<Content>,
    unanswered_tool_calls: Option<Content>,
    answered_tool_parts: Vec<Part>,
) -> Result<(), ActionError> {
    if let Some(partial) = last_partial.as_ref().and_then(model_text_from_partial) {
        save_content(&state.db, session_id, &partial).await?;
        bump_session(&state.db, session_id).await?;
        live_turn::emit(tx, StreamEvent::Final(partial));
        return Ok(());
    }

    if let Some(model) = unanswered_tool_calls {
        let parts = complete_cancelled_tool_parts(&model, answered_tool_parts);
        if !parts.is_empty() {
            let user_tool = Content {
                role: Role::User,
                parts,
            };
            save_content(&state.db, session_id, &user_tool).await?;
            bump_session(&state.db, session_id).await?;
            live_turn::emit(tx, StreamEvent::Tool(user_tool));
        }
    }

    live_turn::emit(tx, StreamEvent::Stopped);
    Ok(())
}

const EMAIL_LOG_TARGET: &str = "llm_assistant::imap";

async fn spawn_reply_email_if_needed(state: &LlmAssistantState, session_id: i64, model: &Content) {
    let Ok(Some(sess)) = SessionEntity::find_by_id(session_id).one(&state.db).await else {
        return;
    };
    let Some(to) = sess
        .reply_email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return;
    };
    let body = content_answer_text(model);
    if body.trim().is_empty() {
        return;
    }
    let Ok(prefs) = super::preferences::load_preferences(&state.db).await else {
        return;
    };
    let subject = if sess.title.trim().is_empty() {
        "Re: Assistant reply".to_string()
    } else {
        format!("Re: {}", sess.title.trim())
    };
    let to = to.to_string();
    let threading = email_send::EmailThreading {
        in_reply_to: sess.email_message_id.clone(),
        references: sess.email_references.clone(),
    };
    tokio::spawn(async move {
        match email_send::send_reply_email(&prefs, &to, &subject, &body, threading).await {
            Ok(()) => tracing::warn!(target: EMAIL_LOG_TARGET, "emailed reply to {to}"),
            Err(e) => tracing::error!(target: EMAIL_LOG_TARGET, "email reply to {to} failed: {e}"),
        }
    });
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

    #[test]
    fn partial_text_drops_thoughts_and_function_calls() {
        let partial = Content {
            role: Role::Model,
            parts: vec![
                Part {
                    thought: true,
                    text: Some("thinking".into()),
                    ..Default::default()
                },
                Part {
                    text: Some("  hello  ".into()),
                    ..Default::default()
                },
                Part {
                    function_call: Some(FunctionCall {
                        name: "list_skills".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
        };
        let model = model_text_from_partial(&partial).expect("text");
        assert_eq!(model.parts.len(), 1);
        assert_eq!(model.parts[0].text.as_deref(), Some("hello"));
        assert!(model.parts[0].function_call.is_none());
    }

    #[test]
    fn partial_without_visible_text_is_none() {
        let partial = Content {
            role: Role::Model,
            parts: vec![Part {
                function_call: Some(FunctionCall {
                    name: "list_skills".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }],
        };
        assert!(model_text_from_partial(&partial).is_none());
    }

    #[test]
    fn cancelled_tool_parts_fill_unanswered_calls() {
        let model = Content {
            role: Role::Model,
            parts: vec![
                Part {
                    function_call: Some(FunctionCall {
                        id: "a".into(),
                        name: "one".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Part {
                    function_call: Some(FunctionCall {
                        id: "b".into(),
                        name: "two".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
        };
        let already = vec![Part {
            function_response: Some(FunctionResponse {
                function_response_id: "a".into(),
                name: "one".into(),
                response: Some(serde_json::json!({ "ok": true })),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let parts = complete_cancelled_tool_parts(&model, already);
        assert_eq!(parts.len(), 2);
        assert_eq!(
            parts[1]
                .function_response
                .as_ref()
                .map(|fr| fr.function_response_id.as_str()),
            Some("b")
        );
        assert_eq!(
            parts[1]
                .function_response
                .as_ref()
                .and_then(|fr| fr.response.as_ref()),
            Some(&serde_json::json!({ "error": "cancelled" }))
        );
    }
}
