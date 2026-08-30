//! Authenticated WebSocket chat — HTMX 4 `hx-ws` + Gemini streaming + tools.
//!
//! Turns outlive a single socket: on disconnect the Gemini/tool loop keeps
//! running; on reconnect the client sends `attach` and resumes live events.

use std::sync::Arc;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use tokio::sync::broadcast;

use crate::{
    http::Cap,
    llm_tools::LlmToolsCapability,
    plugins::{
        filesystem::{node, state::FilesystemState, zip::read_file_bytes},
        llm_assistant::{
            actions::{StreamEvent, run_stream_turn, transcript_html},
            content::{attachments::attachment_part, load_session_contents},
            entities::session::{self, Entity as SessionEntity},
            genai::{Content, Part, Role},
            handlers::history::{load_user_sessions, session_display_title},
            live_turn,
            state::LlmAssistantState,
            templates::modal_sessions_oob,
            ws::{
                UserMessage, assistant_bubble_html, error_oob, final_assistant_oob, form_busy_oob,
                form_ready_oob, session_name_oob, tool_call_inner_html, tool_response_inner_html,
                transcript_replace_oob, user_ack_oob, user_bubble_html, working_append_oob,
                working_close_oob, working_open_oob,
            },
        },
        users::middleware::RequireAuth,
    },
    rune_env::RuneEnvCapability,
};

fn can_access_session(session: &session::Model, user_id: i64, is_superuser: bool) -> bool {
    is_superuser || session.user_id == user_id
}

/// Open or append into the live Tools Called group for this turn.
///
/// `working_ids` holds `(details_id, body_id)` for the open group, if any.
fn working_tool_oob(
    working_ids: &mut Option<(String, String)>,
    working_seq: &mut u64,
    inner: &str,
) -> String {
    if let Some((_, body_id)) = working_ids.as_ref() {
        working_append_oob(body_id, inner)
    } else {
        *working_seq += 1;
        let details_id = format!("llm_assistant_working_details_{}", *working_seq);
        let body_id = format!("llm_assistant_working_body_{}", *working_seq);
        let html = working_open_oob(&details_id, &body_id, inner);
        *working_ids = Some((details_id, body_id));
        html
    }
}

fn is_broken_pipe(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("broken pipe") || lower.contains("connection reset")
}

/// `GET /llm-assistant/ws/` — upgrade after cookie auth.
pub async fn upgrade(
    Cap(state): Cap<LlmAssistantState>,
    Cap(fs): Cap<FilesystemState>,
    Cap(tools): Cap<Arc<LlmToolsCapability>>,
    Cap(rune_env): Cap<Arc<RuneEnvCapability>>,
    RequireAuth(ctx): RequireAuth,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user_id = ctx.user.id;
    let is_superuser = ctx.user.is_superuser;
    let timezone = ctx.timezone.clone();
    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            state,
            fs,
            tools,
            rune_env,
            user_id,
            is_superuser,
            timezone,
        )
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    state: LlmAssistantState,
    fs: FilesystemState,
    tools: Arc<LlmToolsCapability>,
    rune_env: Arc<RuneEnvCapability>,
    user_id: i64,
    is_superuser: bool,
    timezone: String,
) {
    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            Message::Ping(p) => {
                if let Err(e) = socket.send(Message::Pong(p)).await {
                    tracing::warn!(error = %e, "llm_assistant: failed to send ws pong");
                }
                continue;
            }
            _ => continue,
        };

        let user_msg = match UserMessage::from_envelope(&text) {
            Ok(m) => m,
            Err(e) => {
                if let Err(send_err) = socket.send(Message::Text(error_oob(&e).into())).await {
                    tracing::warn!(error = %send_err, "llm_assistant: failed to send ws parse error");
                }
                continue;
            }
        };

        if user_msg.is_attach() {
            match attach_session(
                &mut socket,
                &state,
                user_id,
                is_superuser,
                &timezone,
                user_msg.session_id,
            )
            .await
            {
                Ok(AttachOutcome::Idle) | Ok(AttachOutcome::Completed) => {}
                Ok(AttachOutcome::Detached) => {
                    tracing::debug!("llm_assistant: ws detached during attach");
                    break;
                }
                Err(e) => {
                    if is_broken_pipe(&e) {
                        tracing::debug!(error = %e, "llm_assistant: ws attach disconnected");
                        break;
                    }
                    tracing::warn!(error = %e, "llm_assistant: ws attach failed");
                    if let Err(send_err) = socket.send(Message::Text(error_oob(&e).into())).await {
                        tracing::warn!(error = %send_err, "llm_assistant: failed to send ws attach error");
                        break;
                    }
                }
            }
            continue;
        }

        match process_message(
            &mut socket,
            &state,
            &fs,
            tools.clone(),
            rune_env.clone(),
            user_id,
            is_superuser,
            &timezone,
            user_msg,
        )
        .await
        {
            Ok(TurnOutcome::Completed) => {}
            Ok(TurnOutcome::Detached) => {
                tracing::debug!("llm_assistant: ws detached; turn continues in background");
                break;
            }
            Err(e) => {
                tracing::warn!(error = %e, "llm_assistant: ws turn failed");
                if is_broken_pipe(&e) {
                    break;
                }
                if let Err(send_err) = socket.send(Message::Text(error_oob(&e).into())).await {
                    tracing::warn!(error = %send_err, "llm_assistant: failed to send ws turn error");
                    break;
                }
            }
        }
    }
}

#[derive(Debug)]
enum TurnOutcome {
    Completed,
    /// Client disconnected; turn keeps running for a later attach.
    Detached,
}

#[derive(Debug)]
enum AttachOutcome {
    /// No in-flight turn; transcript refreshed only.
    Idle,
    Completed,
    Detached,
}

/// Catch up transcript from DB and resume live events for an in-flight turn.
async fn attach_session(
    socket: &mut WebSocket,
    state: &LlmAssistantState,
    user_id: i64,
    is_superuser: bool,
    timezone: &str,
    session_id: i64,
) -> Result<AttachOutcome, String> {
    if session_id == 0 {
        return Ok(AttachOutcome::Idle);
    }
    let sess = SessionEntity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "session not found".to_string())?;
    if !can_access_session(&sess, user_id, is_superuser) {
        return Err("session belongs to another user".into());
    }

    // Snapshot DB first, then subscribe so we only receive events after the snapshot.
    let contents = load_session_contents(&state.db, session_id)
        .await
        .map_err(|e| e.to_string())?;
    let live = state.live_turns.contains(session_id);
    let rx = state.live_turns.subscribe(session_id);

    let mut html = transcript_replace_oob(&transcript_html(&contents));
    if live {
        html.push_str(&form_busy_oob());
    } else {
        html.push_str(&form_ready_oob());
    }
    socket
        .send(Message::Text(html.into()))
        .await
        .map_err(|e| e.to_string())?;

    let Some(rx) = rx else {
        return Ok(AttachOutcome::Idle);
    };

    let mut working_ids: Option<(String, String)> = None;
    let mut working_seq: u64 = 0;
    match forward_events(
        socket,
        rx,
        &mut working_ids,
        &mut working_seq,
        ForwardCtx {
            user_id,
            is_superuser,
            timezone,
            state,
            // Attach path: session already exists; skip session-list OOB on UserSaved.
            session_created: false,
            skip_user_saved: true,
        },
    )
    .await
    {
        Ok(()) => Ok(AttachOutcome::Completed),
        Err(e) if is_broken_pipe(&e) => Ok(AttachOutcome::Detached),
        Err(e) => Err(e),
    }
}

struct ForwardCtx<'a> {
    user_id: i64,
    is_superuser: bool,
    timezone: &'a str,
    state: &'a LlmAssistantState,
    session_created: bool,
    /// After reconnect, UserSaved was already shown / is in the refreshed transcript.
    skip_user_saved: bool,
}

async fn process_message(
    socket: &mut WebSocket,
    state: &LlmAssistantState,
    fs: &FilesystemState,
    tools: Arc<LlmToolsCapability>,
    rune_env: Arc<RuneEnvCapability>,
    user_id: i64,
    is_superuser: bool,
    timezone: &str,
    msg: UserMessage,
) -> Result<TurnOutcome, String> {
    let (session_id, session_created) =
        resolve_session(state, user_id, is_superuser, msg.session_id).await?;

    if state.live_turns.contains(session_id) {
        return Err("assistant is still working on this session — wait or reconnect".into());
    }

    let user = build_user_content(fs, &msg).await?;

    let (tx, rx) = live_turn::new_turn_channel();
    state.live_turns.insert(session_id, tx.clone());

    let state_clone = state.clone();
    let store = fs.store.clone();
    let live_turns = state.live_turns.clone();
    let join = tokio::spawn(async move {
        let result =
            run_stream_turn(&state_clone, store, tools, rune_env, session_id, user, tx).await;
        live_turns.remove(session_id);
        result
    });

    let mut working_ids: Option<(String, String)> = None;
    let mut working_seq: u64 = 0;

    let forward = forward_events(
        socket,
        rx,
        &mut working_ids,
        &mut working_seq,
        ForwardCtx {
            user_id,
            is_superuser,
            timezone,
            state,
            session_created,
            skip_user_saved: false,
        },
    )
    .await;

    match forward {
        Ok(()) => {
            join.await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(TurnOutcome::Completed)
        }
        Err(e) if is_broken_pipe(&e) => {
            tracing::debug!(
                session_id,
                error = %e,
                "llm_assistant: ws disconnected mid-turn; will reattach on reconnect"
            );
            Ok(TurnOutcome::Detached)
        }
        Err(e) => {
            let turn_err = join
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r.map_err(|e| e.to_string()));
            match turn_err {
                Ok(()) => Err(e),
                Err(te) => Err(format!("{e}; turn: {te}")),
            }
        }
    }
}

/// Forward stream events to the socket until the turn ends or the socket dies.
async fn forward_events(
    socket: &mut WebSocket,
    mut rx: broadcast::Receiver<StreamEvent>,
    working_ids: &mut Option<(String, String)>,
    working_seq: &mut u64,
    ctx: ForwardCtx<'_>,
) -> Result<(), String> {
    loop {
        match rx.recv().await {
            Ok(ev) => {
                let html = match ev {
                    StreamEvent::UserSaved {
                        session_id,
                        user,
                        title,
                    } => {
                        if ctx.skip_user_saved {
                            continue;
                        }
                        let mut html = user_ack_oob(session_id, &user_bubble_html(&user));
                        if ctx.session_created || title.is_some() {
                            let name =
                                session_display_title(session_id, title.as_deref().unwrap_or(""));
                            html.push_str(&session_name_oob(&name));
                            let sessions = load_user_sessions(
                                &ctx.state.db,
                                ctx.user_id,
                                ctx.is_superuser,
                                ctx.timezone,
                            )
                            .await;
                            html.push_str(&modal_sessions_oob(&sessions).into_string());
                        }
                        html
                    }
                    StreamEvent::Partial(_content) => {
                        // Live stream panel removed; Final/ToolCall/Tool update the transcript.
                        continue;
                    }
                    StreamEvent::ToolCall(content) => {
                        working_tool_oob(working_ids, working_seq, &tool_call_inner_html(&content))
                    }
                    StreamEvent::Tool(content) => working_tool_oob(
                        working_ids,
                        working_seq,
                        &tool_response_inner_html(&content),
                    ),
                    StreamEvent::Final(content) => {
                        let mut html = String::new();
                        if let Some((details_id, _)) = working_ids.take() {
                            html.push_str(&working_close_oob(&details_id));
                        }
                        html.push_str(&final_assistant_oob(&assistant_bubble_html(&content)));
                        html
                    }
                };
                socket
                    .send(Message::Text(html.into()))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    Ok(())
}

/// Resolve an existing session, or create one when `session_id == 0` (first message).
/// Returns `(session_id, created)`.
async fn resolve_session(
    state: &LlmAssistantState,
    user_id: i64,
    is_superuser: bool,
    session_id: i64,
) -> Result<(i64, bool), String> {
    if session_id == 0 {
        let now = Utc::now();
        let model = session::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            title: Set(String::new()),
            user_id: Set(user_id),
            reply_email: Set(None),
            email_message_id: Set(None),
            email_references: Set(None),
        };
        let saved = model.insert(&state.db).await.map_err(|e| e.to_string())?;
        return Ok((saved.id, true));
    }

    let sess = SessionEntity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "session not found".to_string())?;
    if !can_access_session(&sess, user_id, is_superuser) {
        return Err("session belongs to another user".into());
    }
    Ok((sess.id, false))
}

async fn build_user_content(fs: &FilesystemState, msg: &UserMessage) -> Result<Content, String> {
    let mut parts = Vec::new();
    let text = msg.message.trim();
    if !text.is_empty() {
        parts.push(Part {
            text: Some(text.to_string()),
            ..Default::default()
        });
    }

    for file_id in &msg.files {
        let Some(vnode) = node::get_by_id(&fs.db, *file_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            continue;
        };
        if vnode.is_directory {
            continue;
        }
        let bytes = read_file_bytes(fs.store.as_ref(), &vnode)
            .await
            .map_err(|e| e.to_string())?;
        parts.push(attachment_part(&vnode.name, &bytes, Some(vnode.id)));
    }

    if parts.is_empty() {
        return Err("message is empty".into());
    }
    Ok(Content {
        role: Role::User,
        parts,
    })
}
