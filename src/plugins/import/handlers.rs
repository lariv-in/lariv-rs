//! HTTP handlers for import page and XLSX upload.
use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maud::Markup;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    export::ExportCapability,
    html_form::HtmlForm,
    http::Cap,
    plugins::{
        import::{
            forms::ImportForm, state::ImportState, templates::ImportPage, upsert, xlsx,
        },
        users::middleware::RequireStaff,
        users::state::AuthContext,
    },
    web::{Htmx, html_built_page_or_app_layout},
};

const MAX_UPLOAD_BYTES: usize = crate::http::REQUEST_BODY_LIMIT_BYTES;

fn import_page(model_count: i64, error: String, result: Option<upsert::ImportReport>) -> ImportPage {
    ImportPage {
        error,
        result,
        model_count,
    }
}

fn render_page(
    page: &ImportPage,
    htmx: &Htmx,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
) -> Markup {
    html_built_page_or_app_layout(page, htmx, chrome, &SlotCtx::from_auth(ctx))
}

/// HTTP handler: `page`.
pub async fn page(
    Cap(export): Cap<ExportCapability>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
) -> Response {
    let page = import_page(export.catalog().entries.len() as i64, String::new(), None);
    render_page(&page, &htmx, &chrome, &ctx).into_response()
}

/// HTTP handler: `import_post`.
pub async fn import_post(
    Cap(export): Cap<ExportCapability>,
    Cap(state): Cap<ImportState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireStaff(ctx): RequireStaff,
    htmx: Htmx,
    multipart: Multipart,
) -> Response {
    let catalog = export.catalog();
    let model_count = catalog.entries.len() as i64;

    let parsed_form = match ImportForm::from_multipart(multipart).await {
        Ok(form) => form,
        Err(err) => {
            let page = import_page(model_count, err.to_string(), None);
            return (StatusCode::BAD_REQUEST, render_page(&page, &htmx, &chrome, &ctx))
                .into_response();
        }
    };

    let bytes = match parsed_form.file.into_bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            let page = import_page(model_count, err.to_string(), None);
            return (StatusCode::BAD_REQUEST, render_page(&page, &htmx, &chrome, &ctx))
                .into_response();
        }
    };
    if bytes.len() > MAX_UPLOAD_BYTES {
        let page = import_page(
            model_count,
            "xlsx file too large (max 50 MiB)".into(),
            None,
        );
        return (StatusCode::BAD_REQUEST, render_page(&page, &htmx, &chrome, &ctx)).into_response();
    }
    if bytes.is_empty() {
        let page = import_page(model_count, "empty file".into(), None);
        return (StatusCode::BAD_REQUEST, render_page(&page, &htmx, &chrome, &ctx)).into_response();
    }

    let workbook = match xlsx::parse_workbook(&bytes, &catalog) {
        Ok(workbook) => workbook,
        Err(err) => {
            tracing::warn!(error = %err, "import parse failed");
            let page = import_page(model_count, err, None);
            return (StatusCode::BAD_REQUEST, render_page(&page, &htmx, &chrome, &ctx))
                .into_response();
        }
    };

    match upsert::import_workbook(&state.db, &catalog, &workbook).await {
        Ok(report) => {
            let page = import_page(model_count, String::new(), Some(report));
            render_page(&page, &htmx, &chrome, &ctx).into_response()
        }
        Err(err) => {
            tracing::error!(error = %err, "import workbook failed");
            let page = import_page(model_count, err, None);
            (StatusCode::INTERNAL_SERVER_ERROR, render_page(&page, &htmx, &chrome, &ctx))
                .into_response()
        }
    }
}
