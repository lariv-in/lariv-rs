use axum::response::{IntoResponse, Redirect, Response};

use lariv_plugin_finance_accounts::routes::AccountingPreferencesRouteTag;

pub async fn purchase_preferences_get() -> Response {
    Redirect::to(&AccountingPreferencesRouteTag.url()).into_response()
}

pub async fn purchase_preferences_post() -> Response {
    Redirect::to(&AccountingPreferencesRouteTag.url()).into_response()
}
