use std::path::PathBuf;

use axum::{
    body::Body,
    extract::Path as AxumPath,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use maud::{Markup, html};

use lariv_core::components::modal_keyed;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_plugin_filesystem::state::FilesystemState;
use lariv_plugin_users::middleware::RequireAuth;

use crate::forms::InventoryPreferencesForm;
use crate::keys::MovementPdfPreviewModalKey;
use crate::logic::pdf::{MovementPdfError, MovementPdfInput, render_movement_pdf, sample_lines};
use crate::logic::preferences::parse_optional_id;
use crate::movement_type::MovementType;
use crate::routes::MovementPdfPreviewFileRouteTag;

fn preview_cache_dir() -> PathBuf {
    std::env::temp_dir().join("lariv-movement-pdf-preview")
}

fn preview_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}", std::process::id(), nanos)
}

fn is_valid_preview_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 64
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn preview_pdf_path(token: &str) -> PathBuf {
    preview_cache_dir().join(format!("{token}.pdf"))
}

fn store_preview_pdf(token: &str, bytes: &[u8]) -> Result<(), String> {
    let dir = preview_cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create preview cache: {e}"))?;
    std::fs::write(preview_pdf_path(token), bytes).map_err(|e| format!("write preview pdf: {e}"))
}

fn remove_preview_pdf(token: &str) {
    let path = preview_pdf_path(token);
    if let Err(err) = std::fs::remove_file(&path)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(error = %err, path = %path.display(), "failed to remove preview pdf");
    }
}

fn cleanup_stale_previews(max_age_secs: u64) {
    let Ok(read_dir) = std::fs::read_dir(preview_cache_dir()) else {
        return;
    };
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_secs))
        .unwrap_or(std::time::UNIX_EPOCH);
    for entry in read_dir.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified < cutoff
            && let Err(err) = std::fs::remove_file(entry.path())
            && err.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(
                error = %err,
                path = %entry.path().display(),
                "failed to remove stale preview pdf"
            );
        }
    }
}

fn render_preview_modal(pdf_url: &str, error: Option<&str>) -> Markup {
    if let Some(err) = error {
        return modal_keyed::<MovementPdfPreviewModalKey>(
            "max-w-2xl",
            html! {
                h3 class="text-lg font-semibold mb-2" { "Stock movement PDF preview failed" }
                p class="text-error whitespace-pre-wrap" { (err) }
            },
        );
    }
    modal_keyed::<MovementPdfPreviewModalKey>(
        "max-w-6xl w-[95vw]",
        html! {
            h3 class="text-lg font-semibold mb-3" { "Stock movement PDF preview (sample data)" }
            iframe
                src=(pdf_url)
                class="w-full h-[75vh] border border-base-300 rounded bg-white"
                title="Stock movement PDF preview" {}
        },
    )
}

pub async fn preview_in(
    fs: Cap<FilesystemState>,
    ctx: RequireAuth,
    form: HtmlFormBody<InventoryPreferencesForm>,
) -> Markup {
    preview(fs, ctx, form, MovementType::In).await
}

pub async fn preview_out(
    fs: Cap<FilesystemState>,
    ctx: RequireAuth,
    form: HtmlFormBody<InventoryPreferencesForm>,
) -> Markup {
    preview(fs, ctx, form, MovementType::Out).await
}

async fn preview(
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<InventoryPreferencesForm>,
    kind: MovementType,
) -> Markup {
    cleanup_stale_previews(3600);
    let template = match kind {
        MovementType::In => form.movement_in_template.as_str(),
        MovementType::Out => form.movement_out_template.as_str(),
    };
    let number = match kind {
        MovementType::In => "SM-2026-1",
        MovementType::Out => "SM-2026-2",
    };
    let datetime = ctx.format_datetime(chrono::Utc::now()).into_string();
    let input = MovementPdfInput {
        number,
        datetime_label: &datetime,
        movement_type: kind,
        bill_to: crate::logic::party::BillTo::new(false, None, None),
        vehicle_type: "Truck",
        vehicle_number: "MH12AB1234",
        eway_bill: "391234567890",
        driver_id: None,
        lines: sample_lines(),
        template_override: Some(template),
        company_name: Some(&form.company_name),
        company_address: Some(&form.company_address),
        company_phone: Some(&form.company_phone),
        company_email: Some(&form.company_email),
        company_gstin: Some(&form.company_gstin),
        terms: Some(&form.terms_and_conditions),
        logo_vnode_id: Some(parse_optional_id(&form.logo_vnode_id).unwrap_or(0)),
        signature_vnode_id: Some(parse_optional_id(&form.signature_vnode_id).unwrap_or(0)),
        sample: true,
    };
    match render_movement_pdf(&fs, input).await {
        Ok(result) => {
            let token = preview_token();
            if let Err(msg) = store_preview_pdf(&token, &result.bytes) {
                return render_preview_modal("", Some(&msg));
            }
            render_preview_modal(&MovementPdfPreviewFileRouteTag::new(token).url(), None)
        }
        Err(MovementPdfError::Message(msg)) => render_preview_modal("", Some(&msg)),
        Err(MovementPdfError::NotFound) => {
            render_preview_modal("", Some("stock movement not found"))
        }
    }
}

pub async fn pdf_get(
    RequireAuth(_ctx): RequireAuth,
    AxumPath(token): AxumPath<String>,
) -> Response {
    if !is_valid_preview_token(&token) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = preview_pdf_path(&token);
    if !path.starts_with(preview_cache_dir()) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    remove_preview_pdf(&token);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "inline; filename=\"stock-movement-preview.pdf\"".to_string(),
            ),
        ],
        Body::from(bytes),
    )
        .into_response()
}
