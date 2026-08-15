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
    plugins::{
        filesystem::{node, state::FilesystemState, zip::read_file_bytes},
        llm_assistant::{
            actions::{StreamEvent, run_stream_turn},
            entities::session::{self, Entity as SessionEntity},
            genai::{Blob, Content, Part, Role},
            handlers::history::{load_user_sessions, session_display_title},
            state::LlmAssistantState,
            templates::modal_sessions_oob,
            ws::{
                UserMessage, assistant_append_oob, assistant_bubble_html, error_oob,
                final_assistant_oob, session_name_oob, tool_bubble_html, tool_oob, user_ack_oob,
                user_bubble_html,
            },
        },
        users::middleware::RequireAuth,
    },
    rune_env::RuneEnvCapability,
};

fn can_access_session(session: &session::Model, user_id: i64, is_superuser: bool) -> bool {
    is_superuser || session.user_id == user_id
}

/// Guess MIME from filename; if unknown/`octet-stream` and bytes are valid UTF-8, use `text/plain`
/// so Gemini accepts text-like attachments (e.g. `.desktop`).
fn detect_mime(name: &str, bytes: &[u8]) -> String {
    if let Some(mime) = mime_guess::from_path(name).first() {
        let essence = mime.essence_str();
        if essence != "application/octet-stream" {
            return essence.to_string();
        }
    }
    if looks_like_utf8(bytes) {
        return "text/plain".to_string();
    }
    "application/octet-stream".to_string()
}

fn looks_like_utf8(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
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
            &timezone,
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
    timezone: &str,
    msg: UserMessage,
) -> Result<(), String> {
    let (session_id, session_created) =
        resolve_session(state, user_id, is_superuser, msg.session_id).await?;
    let user = build_user_content(fs, &msg).await?;

    let (tx, mut rx) = mpsc::unbounded_channel::<StreamEvent>();
    let state_clone = state.clone();
    let store = fs.store.clone();
    let join = tokio::spawn(async move {
        run_stream_turn(&state_clone, store, tools, rune_env, session_id, user, tx).await
    });

    while let Some(ev) = rx.recv().await {
        match ev {
            StreamEvent::UserSaved { session_id, user } => {
                let mut html = user_ack_oob(session_id, &user_bubble_html(&user));
                if session_created {
                    html.push_str(&session_name_oob(&session_display_title(session_id, "")));
                    let sessions =
                        load_user_sessions(&state.db, user_id, is_superuser, timezone).await;
                    html.push_str(&modal_sessions_oob(&sessions).into_string());
                }
                socket
                    .send(Message::Text(html.into()))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            StreamEvent::Partial(_content) => {
                // Live stream panel removed; Final/ToolCall/Tool update the transcript.
            }
            StreamEvent::ToolCall(content) => {
                // Tool call with args — append, keep Send disabled.
                let html = assistant_append_oob(&assistant_bubble_html(&content));
                socket
                    .send(Message::Text(html.into()))
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
        parts.push(Part {
            inline_data: Some(Blob {
                mime_type: detect_mime(&vnode.name, &bytes),
                data: B64.encode(&bytes),
            }),
            display_name: vnode.name.clone(),
            ..Default::default()
        });
    }

    if parts.is_empty() {
        return Err("message is empty".into());
    }
    Ok(Content {
        role: Role::User,
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::{detect_mime, looks_like_utf8};

    #[test]
    fn desktop_utf8_falls_back_to_text_plain() {
        let body = b"[Desktop Entry]\nName=Test\n";
        assert!(looks_like_utf8(body));
        assert_eq!(detect_mime("app.desktop", body), "text/plain");
    }

    #[test]
    fn known_extension_kept() {
        assert_eq!(detect_mime("photo.png", b"not-really-png"), "image/png");
    }

    #[test]
    fn binary_unknown_stays_octet_stream() {
        let body = [0xff, 0xfe, 0x00, 0x01];
        assert!(!looks_like_utf8(&body));
        assert_eq!(detect_mime("blob.dat", &body), "application/octet-stream");
    }
}
