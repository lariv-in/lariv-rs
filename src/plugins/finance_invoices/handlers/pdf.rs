use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use maud::{Markup, html};

use crate::{
    components::{ButtonDownload, button_download, modal_keyed},
    http::Cap,
    plugins::filesystem::state::FilesystemState,
    plugins::users::middleware::RequireAuth,
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_invoices::logic::invoice_pdf::{
    InvoicePdfError, render_cancelled_invoice_pdf, render_draft_invoice_pdf,
    render_paid_invoice_pdf, render_partially_paid_invoice_pdf, render_posted_invoice_pdf,
};
use crate::plugins::finance_invoices::{
    keys::InvoicePdfModalKey,
    routes::{
        CancelledInvoicePdfRouteTag, DraftInvoicePdfRouteTag, PaidInvoicePdfRouteTag,
        PartiallyPaidInvoicePdfRouteTag, PostedInvoicePdfRouteTag,
    },
    scope::{
        find_active_draft, find_active_paid, find_active_partial, find_active_posted, hub_tab_url,
    },
    state::InvoicesState,
};

fn pdf_error_response(err: InvoicePdfError) -> Response {
    match err {
        InvoicePdfError::NotFound => (StatusCode::NOT_FOUND, "Invoice not found").into_response(),
        InvoicePdfError::Message(msg) if msg.contains("Configure the invoice PDF template") => {
            (StatusCode::BAD_REQUEST, msg).into_response()
        }
        InvoicePdfError::Message(msg) => {
            tracing::error!("invoice pdf: {msg}");
            (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
        }
    }
}

fn pdf_ok_response(
    result: crate::plugins::finance_invoices::logic::invoice_pdf::InvoicePdfResult,
) -> Response {
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

fn render_pdf_modal(title: &str, pdf_url: &str) -> Markup {
    modal_keyed::<InvoicePdfModalKey>(
        "max-w-6xl w-[95vw]",
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

fn render_pdf_modal_error(message: &str) -> Markup {
    modal_keyed::<InvoicePdfModalKey>(
        "max-w-2xl",
        html! {
            h3 class="text-lg font-semibold mb-2" { "Invoice PDF failed" }
            p class="text-error whitespace-pre-wrap" { (message) }
        },
    )
}

/// GET modal: show invoice PDF in an iframe (bytes still served by the `/pdf/` file routes).
pub async fn draft_pdf_modal(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    if find_active_draft(&state.db, id).await.is_none() {
        return render_pdf_modal_error("Draft invoice not found");
    }
    render_pdf_modal(
        &format!("Draft invoice #{id} PDF"),
        &DraftInvoicePdfRouteTag::new(id).path(),
    )
}

pub async fn posted_pdf_modal(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    if find_active_posted(&state.db, id).await.is_none() {
        return render_pdf_modal_error("Posted invoice not found");
    }
    render_pdf_modal(
        &format!("Posted invoice #{id} PDF"),
        &PostedInvoicePdfRouteTag::new(id).path(),
    )
}

pub async fn cancelled_pdf_modal(
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    render_pdf_modal(
        &format!("Cancelled invoice #{id} PDF"),
        &CancelledInvoicePdfRouteTag::new(id).path(),
    )
}

pub async fn paid_pdf_modal(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    if find_active_paid(&state.db, id).await.is_none() {
        return render_pdf_modal_error("Paid invoice not found");
    }
    render_pdf_modal(
        &format!("Paid invoice #{id} PDF"),
        &PaidInvoicePdfRouteTag::new(id).path(),
    )
}

pub async fn partially_paid_pdf_modal(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    if !require_superuser(&ctx) {
        return render_pdf_modal_error("Forbidden");
    }
    if find_active_partial(&state.db, id).await.is_none() {
        return render_pdf_modal_error("Partially paid invoice not found");
    }
    render_pdf_modal(
        &format!("Partially paid invoice #{id} PDF"),
        &PartiallyPaidInvoicePdfRouteTag::new(id).path(),
    )
}

pub async fn draft_pdf(
    Cap(state): Cap<InvoicesState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    match render_draft_invoice_pdf(&fs, id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

pub async fn posted_pdf(
    Cap(state): Cap<InvoicesState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(posted) = find_active_posted(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    };
    match render_posted_invoice_pdf(&fs, posted, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

pub async fn cancelled_pdf(
    Cap(_state): Cap<InvoicesState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match render_cancelled_invoice_pdf(&fs, id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

pub async fn paid_pdf(
    Cap(state): Cap<InvoicesState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if find_active_paid(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("paid")).into_response();
    }
    match render_paid_invoice_pdf(&fs, id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}

pub async fn partially_paid_pdf(
    Cap(state): Cap<InvoicesState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if find_active_partial(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("partial")).into_response();
    }
    match render_partially_paid_invoice_pdf(&fs, id, &ctx.timezone).await {
        Ok(result) => pdf_ok_response(result),
        Err(e) => pdf_error_response(e),
    }
}
