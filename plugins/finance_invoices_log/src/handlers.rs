//! HTTP handlers for one invoice's audit log.

use axum::extract::{Path, Query};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use sea_orm::sea_query::Condition;
use sea_orm::{ColumnTrait, EntityTrait};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, html_built_page_with_slots};
use lariv_plugin_finance_invoices::entities::{
    CancelledInvoiceEntity, DraftInvoiceEntity, PostedInvoiceEntity, QuotationInvoiceEntity,
};
use lariv_plugin_finance_invoices::invoice_detail_menu::InvoiceDetailKind;
use lariv_plugin_logging::entities::log::Column as LogColumn;
use lariv_plugin_logging::handlers::{LogListQuery, load_log_list};
use lariv_plugin_logging::keys::LOG_TABLE;
use lariv_plugin_logging::templates::LogListPage;
use lariv_plugin_users::middleware::RequireAuth;

use crate::inherit::document_line_ids;
use crate::routes::{
    CancelledInvoiceLogListRouteTag, DraftInvoiceLogListRouteTag, PostedInvoiceLogListRouteTag,
    QuotationInvoiceLogListRouteTag,
};
use crate::state::InvoiceLogState;
use crate::templates::InvoiceLogPage;

fn document_log_condition(kind: InvoiceDetailKind, id: i64, line_ids: &[i64]) -> Condition {
    let header = Condition::all()
        .add(LogColumn::TableName.eq(kind.table_name()))
        .add(LogColumn::RowId.eq(id));
    if line_ids.is_empty() {
        return header;
    }
    Condition::any().add(header).add(
        Condition::all()
            .add(LogColumn::TableName.eq(kind.line_table_name()))
            .add(LogColumn::RowId.is_in(line_ids.iter().copied())),
    )
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn filter_form_url(kind: InvoiceDetailKind, id: i64) -> String {
    match kind {
        InvoiceDetailKind::Quotation => QuotationInvoiceLogListRouteTag::new(id).url(),
        InvoiceDetailKind::Draft => DraftInvoiceLogListRouteTag::new(id).url(),
        InvoiceDetailKind::Posted => PostedInvoiceLogListRouteTag::new(id).url(),
        InvoiceDetailKind::Cancelled => CancelledInvoiceLogListRouteTag::new(id).url(),
    }
}

async fn document_number(
    db: &sea_orm::DatabaseConnection,
    kind: InvoiceDetailKind,
    id: i64,
) -> Result<Option<String>, sea_orm::DbErr> {
    match kind {
        InvoiceDetailKind::Quotation => Ok(QuotationInvoiceEntity::find_by_id(id)
            .one(db)
            .await?
            .map(|row| row.number.unwrap_or_default())),
        InvoiceDetailKind::Draft => Ok(DraftInvoiceEntity::find_by_id(id)
            .one(db)
            .await?
            .map(|row| row.number.unwrap_or_default())),
        InvoiceDetailKind::Posted => Ok(PostedInvoiceEntity::find_by_id(id)
            .one(db)
            .await?
            .map(|row| row.number)),
        InvoiceDetailKind::Cancelled => Ok(CancelledInvoiceEntity::find_by_id(id)
            .one(db)
            .await?
            .map(|row| row.number)),
    }
}

struct ListRequest {
    kind: InvoiceDetailKind,
    id: i64,
    state: InvoiceLogState,
    chrome: SharedChromeFolder,
    ctx: lariv_plugin_users::state::AuthContext,
    htmx: Htmx,
    uri: Uri,
    q: LogListQuery,
}

async fn list(request: ListRequest) -> Response {
    let ListRequest {
        kind,
        id,
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
    } = request;
    let number = match document_number(&state.db, kind, id).await {
        Ok(Some(number)) => number,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(error = %error, invoice_id = id, "load invoice for audit log");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let line_ids = match document_line_ids(&state.db, kind, id).await {
        Ok(line_ids) => line_ids,
        Err(error) => {
            tracing::error!(error = %error, invoice_id = id, "load invoice lines for audit log");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let data = load_log_list(
        &state.db,
        &q,
        uri.query().unwrap_or(""),
        &ctx.timezone,
        document_log_condition(kind, id, &line_ids),
        &["table_name", "row_id"],
    )
    .await;
    let page = InvoiceLogPage {
        kind,
        id,
        number,
        logs: LogListPage {
            logs: data.logs,
            fields: data.fields,
            filters: data.filters,
            filter_errors: data.filter_errors,
            sort: data.sort,
            path_and_query: path_and_query(&uri),
            page_size: data.page_size,
            filter_form_url: filter_form_url(kind, id),
        },
    };
    if htmx.targets(LOG_TABLE) {
        return page.logs.render_table().into_response();
    }
    if htmx.wants_main_content() {
        let markup: maud::Markup = page.render_main().into();
        return markup.into_response();
    }
    if htmx.wants_app_layout() {
        let markup: maud::Markup = page.render_pane().into();
        return markup.into_response();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// Audit log for one quotation.
pub async fn quotation(
    Cap(state): Cap<InvoiceLogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<LogListQuery>,
) -> Response {
    list(ListRequest {
        kind: InvoiceDetailKind::Quotation,
        id,
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
    })
    .await
}

/// Audit log for one draft invoice.
pub async fn draft(
    Cap(state): Cap<InvoiceLogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<LogListQuery>,
) -> Response {
    list(ListRequest {
        kind: InvoiceDetailKind::Draft,
        id,
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
    })
    .await
}

/// Audit log for one posted invoice.
pub async fn posted(
    Cap(state): Cap<InvoiceLogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<LogListQuery>,
) -> Response {
    list(ListRequest {
        kind: InvoiceDetailKind::Posted,
        id,
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
    })
    .await
}

/// Audit log for one cancelled invoice.
pub async fn cancelled(
    Cap(state): Cap<InvoiceLogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<LogListQuery>,
) -> Response {
    list(ListRequest {
        kind: InvoiceDetailKind::Cancelled,
        id,
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
    })
    .await
}
