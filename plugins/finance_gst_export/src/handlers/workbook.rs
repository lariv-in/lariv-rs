//! Excel download of posted invoices for GST filing.

use axum::{
    body::Body,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::Utc;

use lariv_core::http::Cap;
use lariv_plugin_users::middleware::RequireAuth;

use crate::state::GstExportState;

pub async fn download(Cap(state): Cap<GstExportState>, RequireAuth(_ctx): RequireAuth) -> Response {
    let bytes = match crate::xlsx::build_invoice_workbook(&state.db).await {
        Ok(bytes) => bytes,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    };
    let filename = format!("invoices_{}.xlsx", Utc::now().format("%Y%m%d_%H%M%S"));
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        Body::from(bytes),
    )
        .into_response()
}
