use axum::response::{IntoResponse, Redirect, Response};

use crate::plugins::finance_accounts::routes::AccountingPreferencesRouteTag;

pub async fn invoice_preferences_get() -> Response {
    Redirect::to(&AccountingPreferencesRouteTag.url()).into_response()
}

pub async fn invoice_preferences_post() -> Response {
    Redirect::to(&AccountingPreferencesRouteTag.url()).into_response()
}

pub async fn payment_preferences_get() -> Response {
    Redirect::to(&AccountingPreferencesRouteTag.url()).into_response()
}

pub async fn payment_preferences_post() -> Response {
    Redirect::to(&AccountingPreferencesRouteTag.url()).into_response()
}
