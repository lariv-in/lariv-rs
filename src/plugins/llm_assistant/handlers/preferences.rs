//! LLM Assistant preferences (Gemini API key/model and Google CSE credentials).

use axum::response::{IntoResponse, Response};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        llm_assistant::{
            config::DEFAULT_CHAT_MODEL,
            entities::LlmAssistantPreferences,
            forms::PreferencesForm,
            preferences::{
                chat_model_or_default, gemini_model_choices, load_preferences, save_preferences,
            },
            state::LlmAssistantState,
            templates::LlmAssistantPreferencesPage,
        },
        users::middleware::RequireStaff,
    },
    web::{Htmx, html_built_page_or_app_layout},
};

async fn prefs_page(
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
    LlmAssistantPreferencesPage {
        api_key: prefs.api_key,
        chat_model,
        chat_model_choices,
        cse_api_key: prefs.cse_api_key,
        cse_cx: prefs.cse_cx,
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
            let page = prefs_page(empty_prefs(), &state.config.chat_model, e.to_string()).await;
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = prefs_page(prefs, &state.config.chat_model, String::new()).await;
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
    let prefs = LlmAssistantPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        api_key: form.api_key.trim().to_string(),
        chat_model: chat_model_or_default(&form.chat_model, &state.config.chat_model),
        cse_api_key: form.cse_api_key.trim().to_string(),
        cse_cx: form.cse_cx.trim().to_string(),
    };

    match save_preferences(&state.db, prefs.clone()).await {
        Ok(_) => {
            let page = prefs_page(prefs, &state.config.chat_model, String::new()).await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
        Err(e) => {
            let page = prefs_page(prefs, &state.config.chat_model, e.to_string()).await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}
