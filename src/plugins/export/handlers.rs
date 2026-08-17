//! HTTP handlers for export page and XLSX download.
use axum::{
    body::Body,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    export::ExportCapability,
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        export::{state::ExportState, templates::ExportPage},
        users::middleware::RequireStaff,
    },
    web::{Htmx, html_built_page_or_app_layout},
};

use super::xlsx;

#[derive(Debug, Deserialize)]
pub struct ExportDownloadForm {
    #[serde(
        default,
        rename = "models",
        deserialize_with = "crate::html_form::form_vec_string"
    )]
    pub models: Vec<String>,
}

/// HTTP handler: `page`.
pub async fn page(
    Cap(export): Cap<ExportCapability>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
) -> Response {
    let catalog = export.catalog();
    let tables: Vec<String> = catalog.entries.iter().map(|e| e.table.clone()).collect();
    let deps_json = serde_json::to_string(
        &catalog
            .entries
            .iter()
            .map(|e| (e.table.clone(), e.immediate_deps.clone()))
            .collect::<std::collections::BTreeMap<_, _>>(),
    )
    .unwrap_or_else(|_| "{}".into());

    let page = ExportPage {
        tables,
        deps_json,
        model_count: catalog.entries.len() as i64,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `download_get`.
pub async fn download_get(RequireStaff(_ctx): RequireStaff) -> axum::response::Redirect {
    axum::response::Redirect::to("/export")
}

/// HTTP handler: `download`.
pub async fn download(
    Cap(export): Cap<ExportCapability>,
    Cap(state): Cap<ExportState>,
    RequireStaff(_ctx): RequireStaff,
    HtmlFormBody(form): HtmlFormBody<ExportDownloadForm>,
) -> Response {
    let selection = match export.expand_selection(&form.models) {
        Ok(selection) => selection,
        Err(err) => {
            tracing::warn!(error = %err, "export selection failed");
            return (StatusCode::BAD_REQUEST, err).into_response();
        }
    };
    let catalog = export.catalog();
    let bytes = match xlsx::build_workbook(&state.db, &catalog, &selection).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!(error = %err, "export workbook failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, err).into_response();
        }
    };
    let filename = format!("export_{}.xlsx", Utc::now().format("%Y%m%d_%H%M%S"));
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(bytes))
        .unwrap()
        .into_response()
}
