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

use crate::keys::MovementPdfModalKey;
use crate::logic::line::{load_in_lines, load_out_lines};
use crate::logic::pdf::{
    MovementPdfError, MovementPdfHeader, movement_pdf_input, render_movement_pdf,
};
use crate::logic::qty::{format_on_hand, type_label};
use crate::movement_type::MovementKind;
use crate::routes::MovementPdfFileRouteTag;
use crate::scope::{find_movement_in, find_movement_out, stock_names};
use crate::state::InventoryState;

fn pdf_error_html(status: StatusCode, message: &str) -> Response {
    let body = html! {
        div style="font-family: system-ui, sans-serif; padding: 1.5rem; max-width: 48rem;" {
            h3 style="font-size: 1.125rem; font-weight: 600; margin: 0 0 0.5rem;" { "Stock movement PDF failed" }
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

fn pdf_error_response(err: MovementPdfError) -> Response {
    match err {
        MovementPdfError::NotFound => {
            (StatusCode::NOT_FOUND, "Stock movement not found").into_response()
        }
        MovementPdfError::Message(msg) => {
            tracing::error!("stock movement pdf: {msg}");
            let status = if msg.contains("template") || msg.contains("Typst compile failed") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            pdf_error_html(status, &msg)
        }
    }
}

fn render_pdf_modal(title: &str, pdf_url: &str) -> Markup {
    modal_keyed::<MovementPdfModalKey>(
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
    modal_keyed::<MovementPdfModalKey>(
        "max-w-2xl",
        html! {
            h3 class="text-lg font-semibold mb-2" { "Stock movement PDF failed" }
            p class="text-error whitespace-pre-wrap" { (message) }
        },
    )
}

pub async fn modal(
    Cap(state): Cap<InventoryState>,
    RequireAuth(_ctx): RequireAuth,
    Path((kind, id)): Path<(String, i64)>,
) -> Markup {
    let Some(kind) = MovementKind::parse(&kind) else {
        return render_pdf_modal_error("Stock movement not found");
    };
    if !kind.has_pdf() {
        return render_pdf_modal_error("Process movements have no PDF");
    }
    let found = match kind {
        MovementKind::In => find_movement_in(&state.db, id).await.is_some(),
        MovementKind::Out => find_movement_out(&state.db, id).await.is_some(),
        MovementKind::Process => false,
    };
    if !found {
        return render_pdf_modal_error("Stock movement not found");
    }
    let title = format!("Stock movement #{id} PDF");
    render_pdf_modal(
        &title,
        &MovementPdfFileRouteTag::new(kind.as_str().to_string(), id).path(),
    )
}

pub async fn file(
    Cap(state): Cap<InventoryState>,
    Cap(fs): Cap<FilesystemState>,
    RequireAuth(ctx): RequireAuth,
    Path((kind, id)): Path<(String, i64)>,
) -> Response {
    let Some(kind) = MovementKind::parse(&kind).filter(|kind| kind.has_pdf()) else {
        return pdf_error_response(MovementPdfError::NotFound);
    };
    match kind {
        MovementKind::In => {
            let Some(movement) = find_movement_in(&state.db, id).await else {
                return pdf_error_response(MovementPdfError::NotFound);
            };
            let lines = pdf_lines(&state.db, &load_in_lines(&state.db, movement.id).await).await;
            let datetime = ctx.format_datetime(movement.datetime).into_string();
            return render_pdf(
                &fs,
                MovementPdfHeader {
                    number: &movement.number,
                    movement_type: kind,
                    bill_to_individual: movement.bill_to_individual,
                    customer_individual: movement.customer_individual,
                    customer_company: movement.customer_company,
                    vehicle_type: movement.vehicle_type.as_deref(),
                    vehicle_number: movement.vehicle_number.as_deref(),
                    eway_bill: movement.eway_bill.as_deref(),
                    driver_id: movement.driver_id,
                },
                &datetime,
                lines,
            )
            .await;
        }
        MovementKind::Out => {
            let Some(movement) = find_movement_out(&state.db, id).await else {
                return pdf_error_response(MovementPdfError::NotFound);
            };
            let lines = pdf_lines(&state.db, &load_out_lines(&state.db, movement.id).await).await;
            let datetime = ctx.format_datetime(movement.datetime).into_string();
            return render_pdf(
                &fs,
                MovementPdfHeader {
                    number: &movement.number,
                    movement_type: kind,
                    bill_to_individual: movement.bill_to_individual,
                    customer_individual: movement.customer_individual,
                    customer_company: movement.customer_company,
                    vehicle_type: movement.vehicle_type.as_deref(),
                    vehicle_number: movement.vehicle_number.as_deref(),
                    eway_bill: movement.eway_bill.as_deref(),
                    driver_id: movement.driver_id,
                },
                &datetime,
                lines,
            )
            .await;
        }
        MovementKind::Process => pdf_error_response(MovementPdfError::NotFound),
    }
}

async fn render_pdf(
    fs: &FilesystemState,
    header: MovementPdfHeader<'_>,
    datetime: &str,
    lines: Vec<crate::logic::pdf::PdfLineInput>,
) -> Response {
    let input = movement_pdf_input(header, datetime, lines);
    match render_movement_pdf(fs, input).await {
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

async fn pdf_lines(
    db: &sea_orm::DatabaseConnection,
    lines: &[crate::logic::line::FormLine],
) -> Vec<crate::logic::pdf::PdfLineInput> {
    let names = stock_names(
        db,
        &lines.iter().map(|line| line.stock_id).collect::<Vec<_>>(),
    )
    .await;
    lines
        .iter()
        .map(|line| crate::logic::pdf::PdfLineInput {
            stock: names.get(&line.stock_id).cloned().unwrap_or_default(),
            qty: format_on_hand(line.qty, ""),
            unit: line.qty_unit.clone(),
            qty_type: type_label(&line.qty_type),
        })
        .collect()
}
