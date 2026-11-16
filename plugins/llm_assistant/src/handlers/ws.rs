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
use tokio_util::sync::CancellationToken;

use crate::llm_tools::LlmToolsCapability;
use crate::{
    actions::{
        StreamEvent, load_api_contents, resolve_context_usage, run_stream_turn,
        session_transcript_html,
    },
    content::attachments::attachment_part,
    context_usage::ContextUsageView,
    entities::session::{self, Entity as SessionEntity},
    genai::{Content, Part, Role, content_thought_text},
    handlers::history::{HistoryListQuery, load_history_page, session_display_title},
    live_turn,
    state::LlmAssistantState,
    templates::modal_sessions_oob,
    ws::{
        UserMessage, assistant_bubble_html, compacted_oob, context_usage_oob, error_notice_oob,
        error_oob, final_assistant_oob, form_busy_oob, form_ready_oob, hitl_pending_inner_html,
        hitl_resolved_oob, session_name_oob, tool_call_inner_html, tool_response_inner_html,
        transcript_replace_oob, user_ack_oob, user_bubble_html, work_append_oob, work_close_oob,
        work_open_oob, work_thought_replace_oob, work_thought_segment_html,
    },
};
use lariv_core::http::Cap;
use lariv_core::rune_env::RuneEnvCapability;
use lariv_plugin_filesystem::{node, state::FilesystemState, zip::read_file_bytes};
use lariv_plugin_users::middleware::RequireAuth;

fn can_access_session(session: &session::Model, user_id: i64, role: &str) -> bool {
    lariv_plugin_users::roles::Superuser::matches(role) || session.user_id == user_id
}

/// Live Work `<details>` for the socket that is forwarding this turn.
///
/// Thoughts and tool activity share one dropdown. Thought tokens replace only the
/// current thought segment so earlier tool rows stay in the body.
struct WorkStream {
    details_id: Option<String>,
    body_id: Option<String>,
    thought_segment_id: Option<String>,
    thought_text: String,
    seq: u64,
    thought_seq: u64,
}

impl WorkStream {
    fn new() -> Self {
        Self {
            details_id: None,
            body_id: None,
            thought_segment_id: None,
            thought_text: String::new(),
            seq: 0,
            thought_seq: 0,
        }
    }

    fn ensure_open(&mut self, inner: &str) -> String {
        if let Some(body_id) = self.body_id.as_ref() {
            return work_append_oob(body_id, inner);
        }
        self.seq += 1;
        let details_id = format!("llm_assistant_work_details_{}", self.seq);
        let body_id = format!("llm_assistant_work_body_{}", self.seq);
        let html = work_open_oob(&details_id, &body_id, inner);
        self.details_id = Some(details_id);
        self.body_id = Some(body_id);
        html
    }

    /// Open, replace, or append a thought segment for a cumulative thought string.
    ///
    /// Returns HTML when the displayed thought changed. Answer-only partials return `None`.
    fn thought_oob(&mut self, content: &Content) -> Option<String> {
        let thought = content_thought_text(content);
        let trimmed = thought.trim();
        if trimmed.is_empty() || trimmed == crate::content::ZWSP || thought == self.thought_text {
            return None;
        }
        let extending = self.thought_segment_id.as_ref().and_then(|segment_id| {
            thought
                .starts_with(self.thought_text.as_str())
                .then(|| segment_id.clone())
        });
        if let Some(segment_id) = extending {
            self.thought_text = thought;
            return Some(work_thought_replace_oob(&segment_id, &self.thought_text));
        }
        self.thought_seq += 1;
        let segment_id = format!("llm_assistant_work_thought_{}", self.thought_seq);
        let segment = work_thought_segment_html(&segment_id, &thought);
        let html = self.ensure_open(&segment);
        self.thought_segment_id = Some(segment_id);
        self.thought_text = thought;
        Some(html)
    }

    /// Append tool / HITL HTML and end the current thought segment.
    fn append_tool(&mut self, inner: &str) -> String {
        self.thought_segment_id = None;
        self.thought_text.clear();
        self.ensure_open(inner)
    }

    fn collapse_oob(&mut self) -> String {
        self.body_id = None;
        self.thought_segment_id = None;
        self.thought_text.clear();
        match self.details_id.take() {
            Some(id) => work_close_oob(&id),
            None => String::new(),
        }
    }
}

fn is_broken_pipe(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("broken pipe")
        || lower.contains("connection reset")
        || lower.contains("connection closed")
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
    let role = ctx.role.clone();
    let timezone = ctx.timezone.clone();
    // The upgrade task does not inherit the request's auth task-local. Without
    // this scope the checkout hook clears `app.user_id` and row-level security
    // rejects the new session.
    ws.on_upgrade(move |socket| {
        lariv_core::db::with_db_actor(
            user_id,
            handle_socket(socket, state, fs, tools, rune_env, user_id, role, timezone),
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
    role: String,
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
                role.as_str(),
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

        if user_msg.is_stop() {
            if !state.live_turns.cancel(user_msg.session_id) {
                if let Err(send_err) = socket.send(Message::Text(form_ready_oob().into())).await {
                    tracing::warn!(error = %send_err, "llm_assistant: failed to send idle stop ack");
                    break;
                }
            }
            continue;
        }

        if user_msg.is_hitl() {
            let approved = user_msg.hitl_approve && !user_msg.hitl_deny;
            let _ = state
                .live_turns
                .resolve_hitl(user_msg.session_id, &user_msg.hitl_id, approved);
            continue;
        }

        match process_message(
            &mut socket,
            &state,
            &fs,
            tools.clone(),
            rune_env.clone(),
            user_id,
            role.as_str(),
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
    role: &str,
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
    if !can_access_session(&sess, user_id, role) {
        return Err("session belongs to another user".into());
    }

    // Snapshot DB first, then subscribe so we only receive events after the snapshot.
    let transcript = session_transcript_html(&state.db, session_id)
        .await
        .map_err(|e| e.to_string())?;
    let api = load_api_contents(&state.db, session_id)
        .await
        .unwrap_or_default();
    let live = state.live_turns.contains(session_id);
    let rx = state.live_turns.subscribe(session_id);

    let mut html = transcript_replace_oob(&transcript);
    let usage = resolve_context_usage(state, Some(&sess), &api).await;
    html.push_str(&context_usage_oob(usage));
    let mut work = WorkStream::new();
    if live {
        html.push_str(&form_busy_oob());
        if let Some(pending) = state.live_turns.pending_hitl(session_id) {
            html.push_str(&work.append_tool(&hitl_pending_inner_html(
                &pending.id,
                &pending.name,
                &pending.args,
            )));
        }
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

    match forward_events(
        socket,
        rx,
        &mut work,
        ForwardCtx {
            session_id,
            user_id,
            role,
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
    session_id: i64,
    user_id: i64,
    role: &'a str,
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
    role: &str,
    timezone: &str,
    msg: UserMessage,
) -> Result<TurnOutcome, String> {
    let (session_id, session_created) =
        resolve_session(state, user_id, role, msg.session_id).await?;

    if state.live_turns.contains(session_id) {
        return Err("assistant is still working on this session — wait or reconnect".into());
    }

    let user = build_user_content(fs, &msg).await?;

    let (tx, rx) = live_turn::new_turn_channel();
    let cancel = CancellationToken::new();
    state
        .live_turns
        .insert(session_id, tx.clone(), cancel.clone());

    let state_clone = state.clone();
    let store = fs.store.clone();
    let live_turns = state.live_turns.clone();
    let actor_id = user_id;
    let join = tokio::spawn(async move {
        lariv_core::db::with_db_actor(actor_id, async move {
            let result = run_stream_turn(
                &state_clone,
                store,
                tools,
                rune_env,
                session_id,
                user,
                tx,
                cancel,
                Some(live_turns.hitl_gate(session_id)),
            )
            .await;
            live_turns.remove(session_id);
            result
        })
        .await
    });

    let mut work = WorkStream::new();

    let forward = forward_events(
        socket,
        rx,
        &mut work,
        ForwardCtx {
            session_id,
            user_id,
            role,
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
    work: &mut WorkStream,
    ctx: ForwardCtx<'_>,
) -> Result<(), String> {
    loop {
        tokio::select! {
            ev = rx.recv() => {
                match ev {
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
                                    let name = session_display_title(
                                        session_id,
                                        title.as_deref().unwrap_or(""),
                                    );
                                    html.push_str(&session_name_oob(&name));
                                    let q = HistoryListQuery::default();
                                    let sessions = load_history_page(
                                        &ctx.state.db,
                                        ctx.user_id,
                                        ctx.role,
                                        &q,
                                        ctx.timezone,
                                    )
                                    .await;
                                    let table = crate::templates::HistoryListPage {
                                        sessions,
                                        path_and_query: crate::routes::HistoryListRouteTag.url(),
                                        page_size: q.page_size.get(),
                                        sort: String::new(),
                                    }
                                    .render_modal_table();
                                    html.push_str(&modal_sessions_oob(table).into_string());
                                }
                                html
                            }
                            StreamEvent::Partial(content) => {
                                match work.thought_oob(&content) {
                                    Some(html) => html,
                                    None => continue,
                                }
                            }
                            StreamEvent::ToolCall(content) => {
                                work.append_tool(&tool_call_inner_html(&content))
                            }
                            StreamEvent::Tool(content) => {
                                work.append_tool(&tool_response_inner_html(&content))
                            }
                            StreamEvent::HitlPending { id, name, args } => {
                                work.append_tool(&hitl_pending_inner_html(&id, &name, &args))
                            }
                            StreamEvent::HitlResolved { id, name, approved } => {
                                hitl_resolved_oob(&id, &name, approved)
                            },
                            StreamEvent::Final(content) => {
                                let mut html = work.collapse_oob();
                                html.push_str(&final_assistant_oob(&assistant_bubble_html(&content)));
                                html
                            }
                            StreamEvent::Compacted { summary } => compacted_oob(&summary),
                            StreamEvent::TurnReady => form_ready_oob(),
                            StreamEvent::Stopped => {
                                let mut html = work.collapse_oob();
                                html.push_str(&form_ready_oob());
                                html
                            }
                            StreamEvent::ContextUsage { used, max } => {
                                context_usage_oob(ContextUsageView::new(used, max))
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
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(p))) => {
                        socket
                            .send(Message::Pong(p))
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => {
                        return Err("connection closed".into());
                    }
                    Some(Ok(Message::Text(t))) => {
                        match UserMessage::from_envelope(&t) {
                            Ok(m) if m.is_stop() => {
                                ctx.state.live_turns.cancel(ctx.session_id);
                            }
                            Ok(m) if m.is_hitl() => {
                                let approved = m.hitl_approve && !m.hitl_deny;
                                let _ = ctx.state.live_turns.resolve_hitl(
                                    ctx.session_id,
                                    &m.hitl_id,
                                    approved,
                                );
                            }
                            Ok(m) if m.is_attach() => {}
                            Ok(_) => {
                                socket
                                    .send(Message::Text(
                                        error_notice_oob(
                                            "assistant is still working on this session — wait or reconnect",
                                        )
                                        .into(),
                                    ))
                                    .await
                                    .map_err(|e| e.to_string())?;
                            }
                            Err(e) => {
                                socket
                                    .send(Message::Text(error_notice_oob(&e).into()))
                                    .await
                                    .map_err(|e| e.to_string())?;
                            }
                        }
                    }
                    Some(Err(e)) => return Err(e.to_string()),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

/// Resolve an existing session, or create one when `session_id == 0` (first message).
/// Returns `(session_id, created)`.
async fn resolve_session(
    state: &LlmAssistantState,
    user_id: i64,
    role: &str,
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
            context_tokens: Set(0),
            is_subagent: Set(false),
        };
        let saved = model.insert(&state.db).await.map_err(|e| e.to_string())?;
        return Ok((saved.id, true));
    }

    let sess = SessionEntity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "session not found".to_string())?;
    if !can_access_session(&sess, user_id, role) {
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
