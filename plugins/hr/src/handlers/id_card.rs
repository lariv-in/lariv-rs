//! Employee ID card PDF.

use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use maud::{Markup, html};

use lariv_core::components::{ButtonDownload, button_download, modal_keyed};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_plugin_filesystem::state::FilesystemState;
use lariv_plugin_users::middleware::RequireAuth;

use crate::forms::LeaveCalcPreferencesForm;
use crate::keys::{EmployeeIdCardModalKey, IdCardPreviewModalKey};
use crate::logic::id_card::{id_card_jwks_json, render_employee_id_card, render_id_card_preview};
use crate::logic::preferences::ensure_id_card_signing_key;
use crate::routes::{EmployeeIdCardPdfRouteTag, HrIdCardPreviewPdfRouteTag};
use crate::scope::find_employee_scoped;
use crate::state::HrState;

fn pdf_error_html(status: StatusCode, message: &str) -> Response {
    let body = html! {
        div style="font-family: system-ui, sans-serif; padding: 1.5rem; max-width: 48rem;" {
            h3 style="font-size: 1.125rem; font-weight: 600; margin: 0 0 0.5rem;" { "ID card failed" }
            pre style="white-space: pre-wrap; color: #b91c1c; margin: 0; font-size: 0.875rem;" { (message) }
        }
    };
    (
        status,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8".to_string())],
        body.into_string(),
    )
        .into_response()
}

fn render_pdf_modal(title: &str, pdf_url: &str) -> Markup {
    modal_keyed::<EmployeeIdCardModalKey>(
        "max-w-3xl w-[95vw]",
        html! {
            div class="flex items-center justify-between gap-3 mb-3 pr-10" {
                h3 class="text-lg font-semibold" { (title) }
                (button_download(ButtonDownload {
                    label: "Download",
                    href: pdf_url,
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
            }
            div class="relative w-full h-[40vh]" x-data="{ loading: true }" {
                div
                    class="absolute inset-0 z-10 flex flex-col items-center justify-center gap-3 rounded border border-base-300 bg-base-100"
                    x-show="loading"
                    x-cloak
                {
                    span class="loading loading-spinner loading-lg" {}
                    p class="text-sm opacity-70" { "Generating PDF…" }
                }
                iframe
                    src=(pdf_url)
                    class="w-full h-full border border-base-300 rounded bg-white"
                    title=(title)
                    x-on:load="loading = false" {}
            }
        },
    )
}

fn render_pdf_modal_error(message: &str) -> Markup {
    modal_keyed::<EmployeeIdCardModalKey>(
        "max-w-2xl",
        html! {
            h3 class="text-lg font-semibold mb-2" { "ID card failed" }
            p class="text-error whitespace-pre-wrap" { (message) }
        },
    )
}

/// GET modal: show the employee ID card PDF in an iframe.
pub async fn modal(
    Cap(state): Cap<HrState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if find_employee_scoped(&state.db, id, &ctx).await.is_none() {
        return render_pdf_modal_error("Employee not found");
    }
    render_pdf_modal("ID Card", &EmployeeIdCardPdfRouteTag::new(id).path())
}

/// GET file: ID card PDF bytes for the iframe and the download button.
pub async fn pdf(
    Cap(state): Cap<HrState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return (StatusCode::NOT_FOUND, "Employee not found").into_response();
    };
    match render_employee_id_card(&fs, &employee, &ctx.timezone).await {
        Ok(result) => {
            let filename = format!("{}.pdf", result.filename_base);
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/pdf".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("inline; filename=\"{filename}\""),
                    ),
                ],
                Body::from(result.bytes),
            )
                .into_response()
        }
        Err(message) => {
            tracing::error!("id card pdf: {message}");
            let status = if message.contains("invalid ID card template")
                || message.contains("rendering ID card template failed")
                || message.contains("Typst compile failed")
            {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            pdf_error_html(status, &message)
        }
    }
}

fn preview_cache_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("lariv-id-card-preview")
}

fn preview_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}

fn is_valid_preview_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 64
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn preview_pdf_path(token: &str) -> std::path::PathBuf {
    preview_cache_dir().join(format!("{token}.pdf"))
}

fn store_preview_pdf(token: &str, bytes: &[u8]) -> Result<(), String> {
    let dir = preview_cache_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("create preview cache: {err}"))?;
    std::fs::write(preview_pdf_path(token), bytes)
        .map_err(|err| format!("write preview pdf: {err}"))
}

fn remove_preview_pdf(token: &str) {
    let path = preview_pdf_path(token);
    if let Err(err) = std::fs::remove_file(&path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(error = %err, path = %path.display(), "failed to remove id card preview pdf");
        }
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
                "failed to remove stale id card preview"
            );
        }
    }
}

fn render_preview_modal(pdf_url: &str, error: Option<&str>) -> Markup {
    if let Some(err) = error {
        return modal_keyed::<IdCardPreviewModalKey>(
            "max-w-2xl",
            html! {
                h3 class="text-lg font-semibold mb-2" { "ID card preview failed" }
                p class="text-error whitespace-pre-wrap" { (err) }
            },
        );
    }
    modal_keyed::<IdCardPreviewModalKey>(
        "max-w-3xl w-[95vw]",
        html! {
            h3 class="text-lg font-semibold mb-3" { "ID card preview" }
            iframe
                src=(pdf_url)
                class="w-full h-[40vh] border border-base-300 rounded bg-white"
                title="ID card preview" {}
        },
    )
}

/// POST from HR preferences: compile the ID card template in the form and open it.
pub async fn preview_post(
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<LeaveCalcPreferencesForm>,
) -> Markup {
    cleanup_stale_previews(3600);
    let template = if form.id_card_format.trim().is_empty() {
        None
    } else {
        Some(form.id_card_format.as_str())
    };
    let signing_key = if form.id_card_signing_key.trim().is_empty() {
        match ensure_id_card_signing_key(&fs.db).await {
            Ok(pem) => pem,
            Err(message) => return render_preview_modal("", Some(&message)),
        }
    } else {
        form.id_card_signing_key.clone()
    };
    match render_id_card_preview(&fs, template, Some(signing_key.as_str())).await {
        Ok(bytes) => {
            let token = preview_token();
            if let Err(message) = store_preview_pdf(&token, &bytes) {
                return render_preview_modal("", Some(&message));
            }
            render_preview_modal(&HrIdCardPreviewPdfRouteTag::new(token).url(), None)
        }
        Err(message) => render_preview_modal("", Some(&message)),
    }
}

/// Serve a cached ID card preview PDF for the modal iframe.
pub async fn preview_pdf(RequireAuth(_ctx): RequireAuth, Path(token): Path<String>) -> Response {
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
                "inline; filename=\"id-card-preview.pdf\"".to_string(),
            ),
        ],
        Body::from(bytes),
    )
        .into_response()
}

/// GET file: ID card public keys as a JWKS document. No authentication.
pub async fn jwks(Cap(state): Cap<HrState>) -> Response {
    match id_card_jwks_json(&state.db).await {
        Ok(body) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/json".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"id-card-public-keys.json\"".to_string(),
                ),
            ],
            Body::from(body),
        )
            .into_response(),
        Err(message) => {
            tracing::error!("id card public keys: {message}");
            (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}
