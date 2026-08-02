use axum::{
    Form,
    body::Body,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    export::ExportCapability,
    http::Cap,
    plugins::{
        export::{state::ExportState, templates::ExportPage},
        users::middleware::{RequireStaff, StaffRejection},
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

pub async fn download(
    Cap(export): Cap<ExportCapability>,
    Cap(state): Cap<ExportState>,
    RequireStaff(_ctx): RequireStaff,
    Form(form): Form<ExportDownloadForm>,
) -> Result<Response, StaffRejection> {
    let selection = export
        .expand_selection(&form.models)
        .map_err(|_| StaffRejection::Forbidden)?;
    let catalog = export.catalog();
    let bytes = xlsx::build_workbook(&state.db, &catalog, &selection)
        .await
        .map_err(|_| StaffRejection::Forbidden)?;
    let filename = format!("export_{}.xlsx", Utc::now().format("%Y%m%d_%H%M%S"));
    Ok(Response::builder()
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
        .into_response())
}
