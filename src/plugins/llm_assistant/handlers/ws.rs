//! Authenticated WebSocket chat — HTMX 4 `hx-ws` + Gemini streaming + tools.

use std::sync::Arc;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use tokio::sync::mpsc;

use crate::{
    http::Cap,
    llm_tools::LlmToolsCapability,
    rune_env::RuneEnvCapability,
    plugins::{
        filesystem::{node, state::FilesystemState, zip::read_file_bytes},
        llm_assistant::{
            actions::{StreamEvent, run_stream_turn},
            entities::session::{self, Entity as SessionEntity},
            genai::{Blob, Content, Part, ROLE_USER},
            state::LlmAssistantState,
            ws::{
                UserMessage, assistant_bubble_html, error_oob, final_assistant_oob,
                stream_inner_html, stream_oob, tool_bubble_html, tool_oob, user_ack_oob,
                user_bubble_html,
            },
        },
        users::middleware::RequireAuth,
    },
};

fn can_access_session(session: &session::Model, user_id: i64, is_superuser: bool) -> bool {
    is_superuser || session.user_id == user_id
}

fn detect_mime(name: &str) -> String {
    mime_guess::from_path(name)
        .first_or_octet_stream()
        .essence_str()
        .to_string()
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
    ws.on_upgrade(move |socket| {
        handle_socket(socket, state, fs, tools, rune_env, user_id, is_superuser)
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
) {
    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            Message::Ping(p) => {
                let _ = socket.send(Message::Pong(p)).await;
                continue;
            }
            _ => continue,
        };

        let user_msg = match UserMessage::from_envelope(&text) {
            Ok(m) => m,
            Err(e) => {
                let _ = socket.send(Message::Text(error_oob(&e).into())).await;
                continue;
            }
        };

        if let Err(e) = process_message(
            &mut socket,
            &state,
            &fs,
            tools.clone(),
            rune_env.clone(),
            user_id,
            is_superuser,
            user_msg,
        )
        .await
        {
            tracing::warn!(error = %e, "llm_assistant: ws turn failed");
            let _ = socket.send(Message::Text(error_oob(&e).into())).await;
            continue;
        }
    }
}

async fn process_message(
    socket: &mut WebSocket,
    state: &LlmAssistantState,
    fs: &FilesystemState,
    tools: Arc<LlmToolsCapability>,
    rune_env: Arc<RuneEnvCapability>,
    user_id: i64,
    is_superuser: bool,
    msg: UserMessage,
) -> Result<(), String> {
    let session_id = resolve_session(state, user_id, is_superuser, msg.session_id).await?;
    let user = build_user_content(fs, &msg).await?;

    let (tx, mut rx) = mpsc::unbounded_channel::<StreamEvent>();
    let state_clone = state.clone();
    let store = fs.store.clone();
    let join = tokio::spawn(async move {
        run_stream_turn(
            &state_clone,
            store,
            tools,
            rune_env,
            session_id,
            user,
            tx,
        )
        .await
    });

    while let Some(ev) = rx.recv().await {
        match ev {
            StreamEvent::UserSaved { session_id, user } => {
                let html = user_ack_oob(session_id, &user_bubble_html(&user));
                socket
                    .send(Message::Text(html.into()))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            StreamEvent::Partial(content) => {
                let inner = stream_inner_html(&content);
                if inner.trim().is_empty() {
                    continue;
                }
                socket
                    .send(Message::Text(stream_oob(&inner).into()))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            StreamEvent::Tool(content) => {
                let html = tool_oob(&tool_bubble_html(&content));
                socket
                    .send(Message::Text(html.into()))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            StreamEvent::Final(content) => {
                let html = final_assistant_oob(&assistant_bubble_html(&content));
                socket
                    .send(Message::Text(html.into()))
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    join.await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

async fn resolve_session(
    state: &LlmAssistantState,
    user_id: i64,
    is_superuser: bool,
    session_id: i64,
) -> Result<i64, String> {
    if session_id == 0 {
        let now = Utc::now();
        let model = session::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            title: Set(String::new()),
            user_id: Set(user_id),
        };
        let saved = model
            .insert(&state.db)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(saved.id);
    }

    let sess = SessionEntity::find_by_id(session_id)
        .one(&state.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "session not found".to_string())?;
    if !can_access_session(&sess, user_id, is_superuser) {
        return Err("session belongs to another user".into());
    }
    Ok(sess.id)
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
        parts.push(Part {
            inline_data: Some(Blob {
                mime_type: detect_mime(&vnode.name),
                data: B64.encode(&bytes),
                display_name: vnode.name.clone(),
            }),
            ..Default::default()
        });
    }

    if parts.is_empty() {
        return Err("message is empty".into());
    }
    Ok(Content {
        role: ROLE_USER.to_string(),
        parts,
    })
}
