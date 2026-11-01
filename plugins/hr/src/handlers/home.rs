//! `GET /dashboard/hr` — open the first HR section this role may use.

use axum::response::Redirect;

use crate::hr_sidebar::first_visible_url;
use crate::routes::HrDashboardGetRouteTag;
use lariv_plugin_users::middleware::RequireAuth;

pub async fn home(RequireAuth(_ctx): RequireAuth) -> Redirect {
    if let Some(url) = first_visible_url() {
        return Redirect::to(&url);
    }
    Redirect::to(&HrDashboardGetRouteTag.url())
}
