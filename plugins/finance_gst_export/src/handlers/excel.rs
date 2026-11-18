use axum::response::{IntoResponse, Response};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_users::middleware::RequireAuth;

use crate::templates::GstExcelPage;

pub async fn page(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    html_built_page_or_app_layout(&GstExcelPage, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
        .into_response()
}
