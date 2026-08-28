//! LLM Assistant preferences (Gemini, Google CSE, and email credentials).

use axum::response::{IntoResponse, Response};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        filesystem::node,
        llm_assistant::{
            config::DEFAULT_CHAT_MODEL,
            entities::LlmAssistantPreferences,
            forms::PreferencesForm,
            preferences::{
                chat_model_or_default, gemini_model_choices, load_preferences,
                mail_encryption_or_default, save_preferences, DEFAULT_MAIL_ENCRYPTION,
            },
            state::LlmAssistantState,
            templates::LlmAssistantPreferencesPage,
        },
        users::{
            entities::user::Entity as UserEntity,
            middleware::RequireStaff,
        },
    },
    web::{Htmx, html_built_page_or_app_layout},
};

async fn email_attachments_parent_display(db: &DatabaseConnection, node_id: Option<i64>) -> String {
    let Some(id) = node_id.filter(|id| *id > 0) else {
        return String::new();
    };
    crate::web::opt_or_log(node::get_by_id(db, id).await, "find email attachments folder")
        .map(|vnode| vnode.name)
        .unwrap_or_default()
}

async fn email_owner_display(db: &DatabaseConnection, user_id: Option<i64>) -> String {
    let Some(id) = user_id.filter(|id| *id > 0) else {
        return String::new();
    };
    crate::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find email owner user")
        .map(|user| {
            let email = user.email.as_str();
            if email.is_empty() {
                user.name
            } else {
                format!("{} ({email})", user.name)
            }
        })
        .unwrap_or_default()
}

async fn prefs_page(
    db: &DatabaseConnection,
    prefs: LlmAssistantPreferences,
    fallback_model: &str,
    error: String,
) -> LlmAssistantPreferencesPage {
    let chat_model = chat_model_or_default(&prefs.chat_model, fallback_model);
    let (chat_model_choices, list_error) = gemini_model_choices(&prefs.api_key, &chat_model).await;
    let error = [error, list_error.unwrap_or_default()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let email_owner_user_id = prefs.email_owner_user_id.unwrap_or(0);
    let email_owner_display = email_owner_display(db, prefs.email_owner_user_id).await;
    let email_attachments_parent_id = prefs.email_attachments_parent_id.unwrap_or(0);
    let email_attachments_parent_display =
        email_attachments_parent_display(db, prefs.email_attachments_parent_id).await;
    LlmAssistantPreferencesPage {
        api_key: prefs.api_key,
        chat_model,
        chat_model_choices,
        cse_api_key: prefs.cse_api_key,
        cse_cx: prefs.cse_cx,
        imap_server: prefs.imap_server,
        imap_port: prefs.imap_port,
        smtp_server: prefs.smtp_server,
        smtp_port: prefs.smtp_port,
        email: prefs.email,
        password: prefs.password,
        mail_encryption: mail_encryption_or_default(&prefs.mail_encryption),
        email_filter: prefs.email_filter,
        email_owner_user_id,
        email_owner_display,
        email_attachments_parent_id,
        email_attachments_parent_display,
        error,
    }
}

fn empty_prefs() -> LlmAssistantPreferences {
    LlmAssistantPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        api_key: String::new(),
        chat_model: DEFAULT_CHAT_MODEL.to_string(),
        cse_api_key: String::new(),
        cse_cx: String::new(),
        imap_server: String::new(),
        imap_port: String::new(),
        smtp_server: String::new(),
        smtp_port: String::new(),
        email: String::new(),
        password: String::new(),
        mail_encryption: DEFAULT_MAIL_ENCRYPTION.to_string(),
        email_filter: String::new(),
        email_owner_user_id: None,
        email_attachments_parent_id: None,
    }
}

/// GET `/llm-assistant/preferences`
pub async fn get(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match load_preferences(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            let page = prefs_page(
                &state.db,
                empty_prefs(),
                &state.config.chat_model,
                e.to_string(),
            )
            .await;
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = prefs_page(&state.db, prefs, &state.config.chat_model, String::new()).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

/// POST `/llm-assistant/preferences`
pub async fn post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<PreferencesForm>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if let Some(owner_id) = form.email_owner_user_id.filter(|id| *id > 0) {
        match UserEntity::find_by_id(owner_id).one(&state.db).await {
            Ok(None) => {
                let page = prefs_page(
                    &state.db,
                    empty_prefs(),
                    &state.config.chat_model,
                    format!("Session owner user not found: {owner_id}"),
                )
                .await;
                return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                    .into_response();
            }
            Ok(Some(_)) => {}
            Err(e) => {
                let page = prefs_page(
                    &state.db,
                    empty_prefs(),
                    &state.config.chat_model,
                    e.to_string(),
                )
                .await;
                return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                    .into_response();
            }
        }
    }

    let existing = load_preferences(&state.db).await.ok();
    let password = if form.password.trim().is_empty() {
        existing
            .as_ref()
            .map(|p| p.password.clone())
            .unwrap_or_default()
    } else {
        form.password
    };
    let prefs = LlmAssistantPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        api_key: form.api_key.trim().to_string(),
        chat_model: chat_model_or_default(&form.chat_model, &state.config.chat_model),
        cse_api_key: form.cse_api_key.trim().to_string(),
        cse_cx: form.cse_cx.trim().to_string(),
        imap_server: form.imap_server.trim().to_string(),
        imap_port: form.imap_port.trim().to_string(),
        smtp_server: form.smtp_server.trim().to_string(),
        smtp_port: form.smtp_port.trim().to_string(),
        email: form.email.trim().to_string(),
        password,
        mail_encryption: mail_encryption_or_default(&form.mail_encryption),
        email_filter: form.email_filter,
        email_owner_user_id: form.email_owner_user_id.filter(|id| *id > 0),
        email_attachments_parent_id: form.email_attachments_parent_id.filter(|id| *id > 0),
    };

    match save_preferences(&state.db, prefs.clone()).await {
        Ok(_) => {
            state.email_listener.restart();
            let page = prefs_page(&state.db, prefs, &state.config.chat_model, String::new()).await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
        Err(e) => {
            let page = prefs_page(&state.db, prefs, &state.config.chat_model, e.to_string()).await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}
