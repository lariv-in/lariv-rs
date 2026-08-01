use axum::{
    Form,
    body::Body,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use frunk::{Generic, hlist};
use serde::Deserialize;

use crate::{
    components::{FoldSlots, SlotCapability, SlotCtx},
    export::ExportCapability,
    http::Cap,
    plugins::{
        export::{state::ExportState, templates::{ExportPage, ExportPageTag}},
        users::middleware::{RequireStaff, StaffRejection},
    },
    template::{RenderAppPane, TemplateCapability, TemplateOf},
    traits::get::GetByTag,
    web::{Htmx, html_page_or_app_layout},
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

pub async fn page<Templates, Slots, Idx, P>(
    Cap(export): Cap<ExportCapability>,
    Cap(_tpl): Cap<TemplateCapability<Templates>>,
    Cap(slots): Cap<SlotCapability<Slots>>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
) -> Response
where
    Slots: FoldSlots + Clone + Send + Sync + 'static,
    Templates: GetByTag<ExportPageTag, Idx, Value = TemplateOf<P>>
        + Clone
        + Send
        + Sync
        + 'static,
    P: Generic<Repr = <ExportPage as Generic>::Repr>
        + lariv_rs::template::RenderTemplate
        + RenderAppPane,
{
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

    html_page_or_app_layout::<P, Slots>(
        &htmx,
        hlist![tables, deps_json, catalog.entries.len() as i64],
        &slots,
        &SlotCtx::from_auth(&ctx),
    )
    .into_response()
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
