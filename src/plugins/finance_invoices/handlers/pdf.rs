use std::collections::HashSet;
use std::io::{Cursor, Write};

use axum::{
    body::Body,
    extract::{Path, Query},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use maud::{Markup, html};
use zip::write::SimpleFileOptions;

use crate::{
    components::{ButtonDownload, button_download, modal_keyed},
    http::Cap,
    plugins::filesystem::state::FilesystemState,
    plugins::users::middleware::RequireAuth,
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_invoices::logic::invoice_pdf::{
    InvoicePdfError, InvoicePdfResult, render_cancelled_invoice_pdf, render_draft_invoice_pdf,
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

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkPdfsQuery {
    #[serde(default)]
    pub ids: Option<String>,
    #[serde(default)]
    pub tab: Option<String>,
}

fn parse_bulk_ids(raw: &str) -> Vec<i64> {
    let mut ids: Vec<i64> = raw
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn allocate_zip_pdf_name(base: &str, used: &mut HashSet<String>) -> String {
    let mut name = format!("{base}.pdf");
    if used.insert(name.clone()) {
        return name;
    }
    let mut n = 2u32;
    loop {
        name = format!("{base}-{n}.pdf");
        if used.insert(name.clone()) {
            return name;
        }
        n = n.saturating_add(1);
    }
}

fn zip_pdfs(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut buf));
        let options = SimpleFileOptions::default();
        for (name, bytes) in entries {
            writer
                .start_file(name, options)
                .map_err(|e| e.to_string())?;
            writer.write_all(bytes).map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf)
}

fn zip_ok_response(filename: &str, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        Body::from(bytes),
    )
        .into_response()
}

async fn render_bulk_tab_pdf(
    state: &InvoicesState,
    fs: &FilesystemState,
    tab: &str,
    id: i64,
    tz: &str,
) -> Result<InvoicePdfResult, InvoicePdfError> {
    match tab {
        "drafts" => {
            if find_active_draft(&state.db, id).await.is_none() {
                return Err(InvoicePdfError::NotFound);
            }
            render_draft_invoice_pdf(fs, id, tz).await
        }
        "posted" => {
            let Some(posted) = find_active_posted(&state.db, id).await else {
                return Err(InvoicePdfError::NotFound);
            };
            render_posted_invoice_pdf(fs, posted, tz).await
        }
        "cancelled" => render_cancelled_invoice_pdf(fs, id, tz).await,
        "paid" => {
            if find_active_paid(&state.db, id).await.is_none() {
                return Err(InvoicePdfError::NotFound);
            }
            render_paid_invoice_pdf(fs, id, tz).await
        }
        "partial" => {
            if find_active_partial(&state.db, id).await.is_none() {
                return Err(InvoicePdfError::NotFound);
            }
            render_partially_paid_invoice_pdf(fs, id, tz).await
        }
        _ => Err(InvoicePdfError::Message(format!("Unknown invoice tab: {tab}"))),
    }
}

/// GET: zip of PDFs for selected hub invoices (`?tab=…&ids=1,2,3`).
pub async fn bulk_pdfs(
    Cap(state): Cap<InvoicesState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkPdfsQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let tab = q.tab.as_deref().unwrap_or("drafts").trim();
    let tab = match tab {
        "posted" | "cancelled" | "paid" | "partial" | "drafts" => tab,
        _ => "drafts",
    };
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    if ids.is_empty() {
        return (StatusCode::BAD_REQUEST, "No invoices selected").into_response();
    }

    let mut entries: Vec<(String, Vec<u8>)> = Vec::with_capacity(ids.len());
    let mut used_names = HashSet::new();
    for id in ids {
        match render_bulk_tab_pdf(&state, &fs, tab, id, &ctx.timezone).await {
            Ok(result) => {
                let name = allocate_zip_pdf_name(&result.filename_base, &mut used_names);
                entries.push((name, result.bytes));
            }
            Err(e) => return pdf_error_response(e),
        }
    }

    match zip_pdfs(&entries) {
        Ok(bytes) => zip_ok_response(&format!("invoices-{tab}.zip"), bytes),
        Err(e) => {
            tracing::error!(error = %e, "failed to build invoice pdf zip");
            (StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
        }
    }
}
