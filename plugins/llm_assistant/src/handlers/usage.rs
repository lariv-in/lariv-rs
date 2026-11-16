//! Token usage calendar page.

use axum::{
    extract::Query,
    response::{IntoResponse, Response},
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;

use crate::{
    state::LlmAssistantState,
    templates::{LlmAssistantUsagePage, token_usage_calendar},
    token_usage::{self as token_ledger, TokenUsageCalendar},
};
use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_users::middleware::RequireAuth;

#[derive(Debug, Deserialize, Default)]
pub struct TokenUsageQuery {
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub month: Option<u32>,
}

async fn load_calendar(
    db: &DatabaseConnection,
    user_id: i64,
    year: Option<i32>,
    month: Option<u32>,
    timezone: &str,
) -> TokenUsageCalendar {
    match token_ledger::load_token_usage_calendar(db, user_id, year, month, timezone).await {
        Ok(calendar) => calendar,
        Err(e) => {
            tracing::warn!(error = %e, "llm_assistant: token usage calendar");
            token_ledger::empty_token_usage_calendar(timezone)
        }
    }
}

/// GET `/llm-assistant/usage`
pub async fn get(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<TokenUsageQuery>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let token_usage = load_calendar(&state.db, ctx.user.id, q.year, q.month, &ctx.timezone).await;
    let page = LlmAssistantUsagePage { token_usage };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

/// GET `/llm-assistant/usage/calendar`
pub async fn calendar(
    Cap(state): Cap<LlmAssistantState>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<TokenUsageQuery>,
) -> Response {
    let calendar = load_calendar(&state.db, ctx.user.id, q.year, q.month, &ctx.timezone).await;
    token_usage_calendar(&calendar).into_response()
}
