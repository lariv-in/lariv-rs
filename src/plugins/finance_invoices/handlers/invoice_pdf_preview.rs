use std::path::PathBuf;

use axum::{
    body::Body,
    extract::Path as AxumPath,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use maud::{Markup, html};

use crate::{
    components::modal::modal_keyed, html_form::HtmlFormBody, http::Cap,
    plugins::filesystem::state::FilesystemState, plugins::users::middleware::RequireAuth,
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_invoices::{
    forms::InvoicePresentationPreferencesForm,
    keys::InvoicePdfPreviewModalKey,
    logic::invoice_pdf::{InvoicePdfError, render_invoice_pdf_preview},
    routes::InvoicePdfPreviewPdfRouteTag,
};

fn preview_cache_dir() -> PathBuf {
    std::env::temp_dir().join("lariv-invoice-pdf-preview")
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
    let path = preview_pdf_path(token);
    std::fs::write(&path, bytes).map_err(|e| format!("write preview pdf: {e}"))
}

fn remove_preview_pdf(token: &str) {
    let path = preview_pdf_path(token);
    if let Err(e) = std::fs::remove_file(&path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(error = %e, path = %path.display(), "failed to remove preview pdf");
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
        if modified < cutoff {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(
                        error = %e,
                        path = %entry.path().display(),
                        "failed to remove stale preview pdf"
                    );
                }
            }
        }
    }
}

fn render_preview_modal(pdf_url: &str, error: Option<&str>) -> Markup {
    if let Some(err) = error {
        return modal_keyed::<InvoicePdfPreviewModalKey>(
            "max-w-2xl",
            html! {
                h3 class="text-lg font-semibold mb-2" { "Invoice PDF preview failed" }
                p class="text-error whitespace-pre-wrap" { (err) }
            },
        );
    }
    modal_keyed::<InvoicePdfPreviewModalKey>(
        "max-w-6xl w-[95vw]",
        html! {
            h3 class="text-lg font-semibold mb-3" { "Invoice PDF preview (sample data)" }
            iframe
                src=(pdf_url)
                class="w-full h-[75vh] border border-base-300 rounded bg-white"
                title="Invoice PDF preview" {}
        },
    )
}

/// POST from accounting preferences — compile sample PDF and open preview modal.
pub async fn modal_post(
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<InvoicePresentationPreferencesForm>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_preview_modal("", Some("Forbidden"));
    }
    cleanup_stale_previews(3600);
    let template = if form.invoice_pdf_template.is_empty() {
        None
    } else {
        Some(form.invoice_pdf_template.as_str())
    };
    match render_invoice_pdf_preview(&fs, template, &ctx.timezone).await {
        Ok(result) => {
            let token = preview_token();
            if let Err(msg) = store_preview_pdf(&token, &result.bytes) {
                return render_preview_modal("", Some(&msg));
            }
            let pdf_url = InvoicePdfPreviewPdfRouteTag::new(token).url();
            render_preview_modal(&pdf_url, None)
        }
        Err(InvoicePdfError::Message(msg)) => render_preview_modal("", Some(&msg)),
        Err(InvoicePdfError::NotFound) => render_preview_modal("", Some("invoice not found")),
    }
}

/// Serve a cached preview PDF inline for the modal iframe.
pub async fn pdf_get(RequireAuth(ctx): RequireAuth, AxumPath(token): AxumPath<String>) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !is_valid_preview_token(&token) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = preview_pdf_path(&token);
    if !path.starts_with(preview_cache_dir()) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    remove_preview_pdf(&token);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "inline; filename=\"invoice-preview.pdf\"".to_string(),
            ),
        ],
        Body::from(bytes),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_token_validation_rejects_path_traversal() {
        assert!(!is_valid_preview_token("../etc/passwd"));
        assert!(!is_valid_preview_token(""));
        assert!(is_valid_preview_token("12345-67890"));
    }

    #[test]
    fn preview_pdf_path_stays_in_cache_dir() {
        let path = preview_pdf_path("abc-123");
        assert!(path.starts_with(preview_cache_dir()));
    }
}
