//! LLM Assistant preferences (Gemini API key).

use axum::{
    Form,
    response::{IntoResponse, Response},
};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::{
        llm_assistant::{
            entities::LlmAssistantPreferences,
            forms::PreferencesForm,
            preferences::{load_preferences, save_preferences},
            routes::PrefsGetRouteTag,
            state::LlmAssistantState,
            templates::LlmAssistantPreferencesPage,
        },
        users::middleware::RequireStaff,
    },
    web::{Htmx, html_built_page_or_app_layout},
};

fn prefs_page(prefs: LlmAssistantPreferences, error: String) -> LlmAssistantPreferencesPage {
    LlmAssistantPreferencesPage {
        api_key: prefs.api_key,
        error,
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
                LlmAssistantPreferences {
                    id: 1,
                    created_at: None,
                    updated_at: None,
                    api_key: String::new(),
                },
                e.to_string(),
            );
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = prefs_page(prefs, String::new());
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

/// POST `/llm-assistant/preferences`
pub async fn post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    Form(form): Form<PreferencesForm>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = LlmAssistantPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        api_key: form.api_key.trim().to_string(),
    };

    match save_preferences(&state.db, prefs.clone()).await {
        Ok(_) => htmx.redirect(PrefsGetRouteTag.url().as_str()),
        Err(e) => {
            let page = prefs_page(prefs, e.to_string());
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}
