//! HTTP handlers for the main LLM chat UI and message posting.

use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use maud::html;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::Deserialize;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::{
        llm_assistant::{
            actions::transcript_html,
            content::load_session_contents,
            entities::session::{self, Entity as SessionEntity},
            handlers::history::load_user_sessions,
            routes::{ChatSessionRouteTag, HistoryListRouteTag},
            state::LlmAssistantState,
            templates::{
                ChatSessionPage, chat_shell, history_sidebar_panel_html, modal_sessions_oob,
                sidebar_chat_partial,
            },
        },
        users::middleware::RequireAuth,
    },
    web::{Htmx, html_built_page_or_app_layout},
};

#[derive(Debug, Deserialize, Default)]
pub struct NewSessionQuery {
    #[serde(default)]
    pub sidebar: Option<String>,
}

/// Assistant index — redirects to history (full-page chat UI removed).
pub async fn index(htmx: Htmx) -> Response {
    htmx.redirect(&HistoryListRouteTag.url())
}

fn is_sidebar_new_session(htmx: &Htmx, sidebar: &NewSessionQuery) -> bool {
    htmx.request
        && sidebar
            .sidebar
            .as_deref()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Create a new session — sidebar HTMX returns OOB list + trigger; full-page form redirects.
pub async fn new_session(
    Cap(state): Cap<LlmAssistantState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    axum::extract::Query(q): axum::extract::Query<NewSessionQuery>,
) -> Response {
    let now = Utc::now();
    let model = session::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(String::new()),
        user_id: Set(ctx.user.id),
        reply_email: Set(None),
        email_message_id: Set(None),
        email_references: Set(None),
    };
    let saved = match model.insert(&state.db).await {
        Ok(s) => s,
        Err(_) => return Redirect::to("/llm-assistant/").into_response(),
    };

    if is_sidebar_new_session(&htmx, &q) {
        let sessions =
            load_user_sessions(&state.db, ctx.user.id, ctx.user.is_superuser, &ctx.timezone).await;
        let body = modal_sessions_oob(&sessions).into_string();
        let trigger = format!(r#"{{"new-session-created": {{"id": {}}}}}"#, saved.id);
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(
                "HX-Trigger",
                HeaderValue::from_str(&trigger).expect("valid HX-Trigger JSON"),
            )
            .body(body.into())
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    htmx.redirect(&ChatSessionRouteTag::new(saved.id).url())
}

fn can_access_session(session: &session::Model, user_id: i64, is_superuser: bool) -> bool {
    is_superuser || session.user_id == user_id
}

fn parse_open_session_id(url: Option<&str>) -> i64 {
    let Some(url) = url else {
        return 0;
    };
    let path = url.split('?').next().unwrap_or(url).trim_end_matches('/');
    let prefix = "/llm-assistant/c/";
    let Some(rest) = path.strip_prefix(prefix) else {
        return 0;
    };
    rest.parse().unwrap_or(0)
}

fn open_session_from_htmx(htmx: &Htmx) -> i64 {
    parse_open_session_id(htmx.current_url.as_deref())
}

fn session_name(sess: &session::Model, id: i64) -> String {
    crate::plugins::llm_assistant::handlers::history::session_display_title(id, &sess.title)
}

fn draft_compact_chat() -> maud::Markup {
    html! {
        div class="flex-1 overflow-hidden min-h-0" {
            (chat_shell(None, "", "", "", true))
        }
    }
}

async fn compact_chat_for_session(state: &LlmAssistantState, id: i64, title: &str) -> maud::Markup {
    let contents = load_session_contents(&state.db, id)
        .await
        .unwrap_or_default();
    let transcript = transcript_html(&contents);
    html! {
        div class="flex-1 overflow-hidden min-h-0" {
            (chat_shell(Some(id), title, &transcript, "", true))
        }
    }
}

/// Prefer the URL session when accessible; otherwise the most recently updated session.
/// Returns `0` when the user has no conversations (draft chat until first message).
async fn resolve_sidebar_session_id(
    state: &LlmAssistantState,
    open_id: i64,
    sessions: &[(i64, String)],
    user_id: i64,
    is_superuser: bool,
) -> i64 {
    if open_id != 0 {
        if let Ok(Some(sess)) = SessionEntity::find_by_id(open_id).one(&state.db).await {
            if can_access_session(&sess, user_id, is_superuser) {
                return open_id;
            }
        }
    }
    sessions.first().map(|(id, _)| *id).unwrap_or(0)
}

/// Full history sidebar panel (lazy-loaded into right drawer).
pub async fn history_panel(
    Cap(state): Cap<LlmAssistantState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    let open_id = open_session_from_htmx(&htmx);
    let sessions =
        load_user_sessions(&state.db, ctx.user.id, ctx.user.is_superuser, &ctx.timezone).await;

    let resolved_id = resolve_sidebar_session_id(
        &state,
        open_id,
        &sessions,
        ctx.user.id,
        ctx.user.is_superuser,
    )
    .await;

    let (active_name, initial_chat) = if resolved_id != 0 {
        if let Ok(Some(sess)) = SessionEntity::find_by_id(resolved_id).one(&state.db).await {
            if can_access_session(&sess, ctx.user.id, ctx.user.is_superuser) {
                let name = session_name(&sess, resolved_id);
                let chat = compact_chat_for_session(&state, resolved_id, &name).await;
                (name, chat)
            } else {
                (String::new(), draft_compact_chat())
            }
        } else {
            (String::new(), draft_compact_chat())
        }
    } else {
        (String::new(), draft_compact_chat())
    };

    history_sidebar_panel_html(&active_name, resolved_id, initial_chat, &sessions).into_response()
}

/// Sidebar chat partial — OOB session name + compact chat shell.
/// `id == 0` returns an empty draft chatbox (no DB session until the first message).
pub async fn sidebar_session(
    Cap(state): Cap<LlmAssistantState>,
    RequireAuth(ctx): RequireAuth,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Response {
    if id == 0 {
        return sidebar_chat_partial("", chat_shell(None, "", "", "", true)).into_response();
    }

    let Some(sess) = crate::web::opt_or_log(
        SessionEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return (StatusCode::NOT_FOUND, "session not found").into_response();
    };
    if !can_access_session(&sess, ctx.user.id, ctx.user.is_superuser) {
        return (StatusCode::FORBIDDEN, "forbidden").into_response();
    }

    let title = session_name(&sess, id);
    let contents = load_session_contents(&state.db, id)
        .await
        .unwrap_or_default();
    let transcript = transcript_html(&contents);
    let chat = chat_shell(Some(id), &title, &transcript, "", true);
    sidebar_chat_partial(&title, chat).into_response()
}

/// Session chat — transcript from Content parts; streaming via WebSocket.
pub async fn session(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Response {
    let Some(sess) = crate::web::opt_or_log(
        SessionEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/llm-assistant/").into_response();
    };
    if !can_access_session(&sess, ctx.user.id, ctx.user.is_superuser) {
        return Redirect::to("/llm-assistant/").into_response();
    }

    let title = session_name(&sess, id);

    let contents = load_session_contents(&state.db, id)
        .await
        .unwrap_or_default();
    let transcript = transcript_html(&contents);
    let page = ChatSessionPage {
        id,
        title,
        transcript_html: transcript,
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::llm_assistant::templates::{modal_sessions_oob, sidebar_chat_partial};
    use crate::web::Htmx;

    #[test]
    fn parse_open_session_id_from_chat_url() {
        assert_eq!(parse_open_session_id(Some("/llm-assistant/c/42/")), 42);
        assert_eq!(
            parse_open_session_id(Some("/llm-assistant/c/42/?foo=1")),
            42
        );
        assert_eq!(parse_open_session_id(Some("/llm-assistant/")), 0);
        assert_eq!(parse_open_session_id(None), 0);
    }

    #[test]
    fn sidebar_new_session_detects_query_flag() {
        let htmx = Htmx {
            request: true,
            ..Default::default()
        };
        assert!(is_sidebar_new_session(
            &htmx,
            &NewSessionQuery {
                sidebar: Some("1".into()),
            },
        ));
        assert!(!is_sidebar_new_session(&htmx, &NewSessionQuery::default()));
        assert!(!is_sidebar_new_session(
            &Htmx::default(),
            &NewSessionQuery {
                sidebar: Some("1".into()),
            },
        ));
    }

    #[test]
    fn modal_sessions_oob_contains_swap_target() {
        let html = modal_sessions_oob(&[(1, "#1 · hello".into())]).into_string();
        assert!(html.contains("modal-sessions-list"));
        assert!(html.contains("hx-swap-oob"));
        assert!(html.contains("sidebar-chat/1/"));
    }

    #[test]
    fn sidebar_chat_partial_oob_session_name() {
        let html = sidebar_chat_partial("My chat", maud::html! { p { "body" } }).into_string();
        assert!(html.contains("session-name-container"));
        assert!(html.contains("hx-swap-oob"));
        assert!(html.contains("My chat"));
    }

    #[test]
    fn history_sidebar_keeps_draft_when_none_active() {
        let html = history_sidebar_panel_html("", 0, draft_compact_chat(), &[]).into_string();
        assert!(!html.contains("htmx.ajax('POST', '/llm-assistant/new-session/?sidebar=1'"));
        assert!(html.contains("openDraft()"));
        assert!(html.contains("/llm-assistant/sidebar-chat/0/"));
        assert!(html.contains("llm_assistant_chat_form"));
        assert!(html.contains("llm-assistant-session-opened"));
        assert!(html.contains("llm-assistant-open-session"));
        assert!(html.contains("loadSession"));
        // Server owns initial selection — no client-side reload of a stale persisted id.
        assert!(html.contains("this.activeSessionId = 0;"));
        assert!(!html.contains("this.$nextTick"));
    }

    #[test]
    fn history_sidebar_opens_server_session_on_init() {
        let html = history_sidebar_panel_html(
            "Recent",
            42,
            draft_compact_chat(),
            &[(42, "#42 · Recent".into())],
        )
        .into_string();
        assert!(html.contains("this.activeSessionId = 42;"));
    }

    #[test]
    fn chat_shell_always_renders_chatbox() {
        let with_session = chat_shell(Some(7), "Hi", "", "", true).into_string();
        let draft = chat_shell(None, "", "", "", true).into_string();
        assert!(with_session.contains("llm_assistant_chat_form"));
        assert!(draft.contains("llm_assistant_chat_form"));
        assert!(draft.contains(r#"name="session_id" value="0""#));
        assert!(draft.contains("hx-ws:connect"));
        assert!(draft.contains(
            r#"<button id="llm_assistant_chat_send" type="submit" class="btn btn-primary">Send</button>"#
        ));
    }
}
