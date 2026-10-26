use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use maud::{Markup, html};

use lariv_core::components::{ButtonDownload, button_download, modal_keyed};
use lariv_core::http::Cap;
use lariv_plugin_filesystem::state::FilesystemState;
use lariv_plugin_users::middleware::RequireAuth;

use sea_orm::EntityTrait;

use crate::entities::quotation::Entity as QuotationEntity;
use crate::keys::QuotationPdfModalKey;
use crate::logic::mail::build_quotation_eml;
use crate::logic::pdf::{PdfError, render_quotation_pdf};
use crate::routes::{QuotationMailRouteTag, QuotationPdfRouteTag};
use crate::state::QuotationsState;

fn pdf_error_html(status: StatusCode, message: &str) -> Response {
    // Inline styles: this document is loaded in the PDF iframe, so app CSS is not applied.
    let body = html! {
        div style="font-family: system-ui, sans-serif; padding: 1.5rem; max-width: 48rem;" {
            h3 style="font-size: 1.125rem; font-weight: 600; margin: 0 0 0.5rem;" { "Quotation PDF failed" }
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

fn pdf_error_response(err: PdfError) -> Response {
    match err {
        PdfError::NotFound => (StatusCode::NOT_FOUND, "Quotation not found").into_response(),
        PdfError::Message(msg) => {
            tracing::error!("quotation pdf: {msg}");
            pdf_error_html(StatusCode::BAD_REQUEST, &msg)
        }
    }
}

fn render_pdf_modal(title: &str, pdf_url: &str, mail_url: &str) -> Markup {
    modal_keyed::<QuotationPdfModalKey>(
        "max-w-6xl w-[95vw]",
        html! {
            div class="flex items-center justify-between gap-3 mb-3 pr-10" {
                h3 class="text-lg font-semibold" { (title) }
                div class="flex gap-2" {
                    (button_download(ButtonDownload {
                        label: "Download PDF",
                        href: pdf_url,
                        classes: "btn-outline btn-sm",
                        ..Default::default()
                    }))
                    (button_download(ButtonDownload {
                        label: "Email",
                        href: mail_url,
                        classes: "btn-outline btn-sm",
                        ..Default::default()
                    }))
                }
            }
            div class="relative w-full h-[75vh]" x-data="{ loading: true }" {
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

pub async fn pdf_modal(
    Cap(state): Cap<QuotationsState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    let found = QuotationEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten();
    let Some(quotation) = found else {
        return modal_keyed::<QuotationPdfModalKey>(
            "max-w-2xl",
            html! {
                h3 class="text-lg font-semibold mb-2" { "Quotation PDF failed" }
                p class="text-error" { "Quotation not found" }
            },
        );
    };
    let title = if quotation.number.trim().is_empty() {
        format!("Quotation #{id}")
    } else {
        format!("Quotation {}", quotation.number)
    };
    render_pdf_modal(
        &title,
        &QuotationPdfRouteTag::new(id).url(),
        &QuotationMailRouteTag::new(id).url(),
    )
}

pub async fn pdf_file(
    Cap(state): Cap<QuotationsState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    match render_quotation_pdf(&state.db, Some(&fs), id).await {
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
        Err(err) => pdf_error_response(err),
    }
}

pub async fn mail_file(
    Cap(state): Cap<QuotationsState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    match build_quotation_eml(&state.db, Some(&fs), id).await {
        Ok(eml) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "message/rfc822".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", eml.filename),
                ),
            ],
            Body::from(eml.bytes),
        )
            .into_response(),
        Err(err) => pdf_error_response(err),
    }
}
