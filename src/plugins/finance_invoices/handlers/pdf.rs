use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};

use crate::{http::Cap, plugins::filesystem::state::FilesystemState, plugins::users::middleware::RequireAuth};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_invoices::logic::invoice_pdf::{
    InvoicePdfError, render_cancelled_invoice_pdf, render_draft_invoice_pdf,
    render_paid_invoice_pdf, render_partially_paid_invoice_pdf, render_posted_invoice_pdf,
};
use crate::plugins::finance_invoices::{scope::{
        find_active_draft, find_active_paid, find_active_partial, find_active_posted, hub_tab_url,
    }, state::InvoicesState};

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

fn pdf_ok_response(result: crate::plugins::finance_invoices::logic::invoice_pdf::InvoicePdfResult) -> Response {
    let filename = format!("{}.pdf", result.filename_base);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        Body::from(result.bytes),
    )
        .into_response()
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
