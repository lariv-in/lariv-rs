use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    plugins::users::middleware::RequireAuth,
    template::RenderAppPane,
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done,
    },
};

use crate::plugins::finance_accounts::scope::{
    CurrencyFormat, load_account_parent_label, load_journal_entry_currency_format,
    load_journal_entry_currency_formats,
};
use crate::plugins::finance_common::require_superuser;
use crate::plugins::finance_taxes::scope::{load_taxes_by_ids, tax_label};

use crate::plugins::finance_invoices::{
    entities::{
        payment::{self, Entity as PaymentEntity},
        payment_batch::{self, Entity as PaymentBatchEntity},
        posted_invoice::{self, Entity as PostedInvoiceEntity},
    },
    forms::PaymentForm,
    handlers::ModalNameQuery,
    keys::{
        PaymentCreateModalKey, PaymentTableKey, PostedInvoiceSelectModalKey,
        PostedInvoiceSelectTableKey,
    },
    logic::{
        CreatePaymentInput, create_payment, format_invoice_date,
        invoice_line_editor::invoice_header_tax_labels, parse_invoice_datetime,
        parse_payment_amount, posted_invoice_open_balance, tax_assoc::load_payment_tax_ids,
    },
    routes::{
        PaidInvoiceDetailRouteTag, PartiallyPaidInvoiceDetailRouteTag, PaymentBatchDetailRouteTag,
        PostedInvoiceDetailRouteTag,
    },
    scope::sql_posted_not_cancelled,
    state::InvoicesState,
    templates::{
        PaymentBatchRow, PaymentCreateModalPage, PaymentDetailPage, PaymentListPage, PaymentRow,
        PostedInvoiceSelectPage, PostedInvoiceSelectRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct ListQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub target_input: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct PaymentCreateQuery {
    #[serde(flatten)]
    pub modal: ModalNameQuery,
    #[serde(default, rename = "PostedInvoiceID")]
    pub posted_invoice_id: Option<i64>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn posted_invoice_display_label(id: i64, number: &str) -> String {
    if number.trim().is_empty() {
        format!("#{id}")
    } else {
        format!("{number} (#{id})")
    }
}

async fn load_payment_form_context(
    db: &sea_orm::DatabaseConnection,
    posted_invoice_id: i64,
    account_id: &str,
    tax_ids: &[i64],
) -> (String, String, Vec<ManyToManyItem>) {
    let posted_invoice_display = if posted_invoice_id > 0 {
        crate::web::opt_or_log(
            PostedInvoiceEntity::find_by_id(posted_invoice_id)
                .one(db)
                .await,
            "find by id",
        )
        .map(|inv| posted_invoice_display_label(inv.id, &inv.number))
        .unwrap_or_default()
    } else {
        String::new()
    };

    let account_display = if let Ok(id) = account_id.trim().parse::<i64>() {
        load_account_parent_label(db, Some(id)).await
    } else {
        String::new()
    };

    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let tax_items = taxes
        .iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect();

    (posted_invoice_display, account_display, tax_items)
}

async fn payment_create_modal_page(
    db: &sea_orm::DatabaseConnection,
    q: &PaymentCreateQuery,
    form: PaymentForm,
    error: String,
) -> PaymentCreateModalPage {
    let posted_invoice_id = form.posted_invoice_id;
    let (posted_invoice_display, account_display, tax_items) =
        load_payment_form_context(db, posted_invoice_id, &form.account_id, &form.taxes).await;
    PaymentCreateModalPage {
        form_name: q.modal.form_name(),
        refresh_table: q.modal.refresh_table(),
        form,
        posted_invoice_display,
        account_display,
        tax_items,
        error,
    }
}

async fn load_posted_invoice_link(
    db: &sea_orm::DatabaseConnection,
    posted_invoice_id: i64,
) -> (String, Option<String>) {
    if posted_invoice_id <= 0 {
        return (String::new(), None);
    }
    if let Ok(Some(inv)) = PostedInvoiceEntity::find_by_id(posted_invoice_id)
        .one(db)
        .await
    {
        (
            posted_invoice_display_label(inv.id, &inv.number),
            Some(PostedInvoiceDetailRouteTag::new(inv.id).url()),
        )
    } else {
        (format!("#{posted_invoice_id}"), None)
    }
}

async fn query_single_payment_rows(
    db: &sea_orm::DatabaseConnection,
    page_num: u32,
    page_size: u32,
    timezone: &str,
    sort: Option<&str>,
) -> (ObjectList<PaymentRow>, ObjectList<PaymentBatchRow>) {
    let mut query = PaymentEntity::find().filter(payment::Column::PaymentBatchId.is_null());
    let sort = sort.unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Amount DESC") => query.order_by_desc(payment::Column::Amount),
        s if s.eq_ignore_ascii_case("Amount ASC") || s.eq_ignore_ascii_case("Amount") => {
            query.order_by_asc(payment::Column::Amount)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => query.order_by_desc(payment::Column::Datetime),
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(payment::Column::Datetime)
        }
        _ => query.order_by_desc(payment::Column::Id),
    };
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let invoice_ids: Vec<i64> = models.iter().map(|p| p.posted_invoice_id).collect();
    let invoice_labels = if invoice_ids.is_empty() {
        HashMap::new()
    } else {
        PostedInvoiceEntity::find()
            .filter(posted_invoice::Column::Id.is_in(invoice_ids))
            .all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|inv| {
                let label = posted_invoice_display_label(inv.id, &inv.number);
                (inv.id, label)
            })
            .collect()
    };
    let je_ids: Vec<i64> = models.iter().map(|p| p.journal_entry_id).collect();
    let currency_fmts = load_journal_entry_currency_formats(db, &je_ids).await;
    let fallback = CurrencyFormat::fallback();
    let rows: Vec<PaymentRow> = models
        .into_iter()
        .map(|p| {
            let fmt = currency_fmts.get(&p.journal_entry_id).unwrap_or(&fallback);
            PaymentRow {
                id: p.id,
                invoice_label: invoice_labels
                    .get(&p.posted_invoice_id)
                    .cloned()
                    .unwrap_or_else(|| "—".into()),
                amount: fmt.display(p.amount),
                datetime: crate::datetime::DatetimeLabel::short(p.datetime, timezone).into_string(),
            }
        })
        .collect();
    (
        ObjectList::from_page(rows, page_num, page_size, total),
        ObjectList::from_page(Vec::<PaymentBatchRow>::new(), 1, page_size, 0),
    )
}

async fn query_batch_payment_rows(
    db: &sea_orm::DatabaseConnection,
    page_num: u32,
    page_size: u32,
    timezone: &str,
    sort: Option<&str>,
) -> (ObjectList<PaymentRow>, ObjectList<PaymentBatchRow>) {
    let mut query = PaymentBatchEntity::find();
    let sort = sort.unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(payment_batch::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(payment_batch::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Total DESC") => {
            query.order_by_desc(payment_batch::Column::TotalAmount)
        }
        s if s.eq_ignore_ascii_case("Total ASC") || s.eq_ignore_ascii_case("Total") => {
            query.order_by_asc(payment_batch::Column::TotalAmount)
        }
        _ => query.order_by_desc(payment_batch::Column::Id),
    };
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();

    let batch_ids: Vec<i64> = models.iter().map(|b| b.id).collect();
    let payment_counts: HashMap<i64, u64> = if batch_ids.is_empty() {
        HashMap::new()
    } else {
        PaymentEntity::find()
            .filter(payment::Column::PaymentBatchId.is_in(batch_ids))
            .all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .fold(HashMap::new(), |mut acc, p| {
                if let Some(batch_id) = p.payment_batch_id {
                    *acc.entry(batch_id).or_insert(0) += 1;
                }
                acc
            })
    };

    let je_ids: Vec<i64> = models.iter().map(|b| b.journal_entry_id).collect();
    let currency_fmts = load_journal_entry_currency_formats(db, &je_ids).await;
    let fallback = CurrencyFormat::fallback();
    let rows: Vec<PaymentBatchRow> = models
        .into_iter()
        .map(|b| {
            let fmt = currency_fmts.get(&b.journal_entry_id).unwrap_or(&fallback);
            PaymentBatchRow {
                id: b.id,
                datetime: crate::datetime::DatetimeLabel::short(b.datetime, timezone).into_string(),
                total_amount: fmt.display(b.total_amount),
                payment_count: payment_counts.get(&b.id).copied().unwrap_or(0),
            }
        })
        .collect();

    (
        ObjectList::from_page(Vec::<PaymentRow>::new(), 1, page_size, 0),
        ObjectList::from_page(rows, page_num, page_size, total),
    )
}

pub async fn list(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ListQuery>,
) -> maud::Markup {
    let tab = match q.tab.as_deref() {
        Some("batches") => "batches",
        _ => "single",
    };
    let page_num = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.get();
    let (payments, batches) = if tab == "batches" {
        query_batch_payment_rows(
            &state.db,
            page_num,
            page_size,
            &ctx.timezone,
            q.sort.as_deref(),
        )
        .await
    } else {
        query_single_payment_rows(
            &state.db,
            page_num,
            page_size,
            &ctx.timezone,
            q.sort.as_deref(),
        )
        .await
    };
    let page = PaymentListPage {
        tab: tab.to_string(),
        payments,
        batches,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<PaymentTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn create_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<PaymentCreateQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-invoices/payments/").into_response();
    }
    let posted_invoice_id = q.posted_invoice_id.filter(|id| *id > 0).unwrap_or(0);
    // Form input: plain number (no currency symbol).
    let amount = if posted_invoice_id > 0 {
        let open = posted_invoice_open_balance(&state.db, posted_invoice_id)
            .await
            .unwrap_or(rust_decimal::Decimal::ZERO);
        if open > rust_decimal::Decimal::ZERO {
            crate::plugins::finance_common::decimal::decimal_display(open)
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let page = payment_create_modal_page(
        &state.db,
        &q,
        PaymentForm {
            posted_invoice_id,
            amount,
            account_id: String::new(),
            datetime: ctx.datetime_local_input(Utc::now()).into_string(),
            taxes: vec![],
        },
        String::new(),
    )
    .await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<PaymentCreateQuery>,
    HtmlFormBody(form): HtmlFormBody<PaymentForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-invoices/payments/").into_response();
    }
    let posted_invoice_id = form.posted_invoice_id;
    let amount = match parse_payment_amount(&form.amount) {
        Ok(a) => a,
        Err(e) => {
            let page = payment_create_modal_page(&state.db, &q, form, e.to_string()).await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let account_id = form
        .account_id
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0);
    let input = CreatePaymentInput {
        posted_invoice_id,
        amount,
        account_id,
        datetime: parse_invoice_datetime(&form.datetime, &ctx.timezone),
        withholding_tax_ids: form.taxes.clone(),
    };
    match create_payment(&state.db, input).await {
        Ok(result) => {
            let detail_url = if result.is_full {
                PaidInvoiceDetailRouteTag::new(result.settlement_id).url()
            } else {
                PartiallyPaidInvoiceDetailRouteTag::new(result.settlement_id).url()
            };
            respond_create_modal_done::<PaymentCreateModalKey>(
                &htmx,
                &q.modal.refresh_table(),
                &detail_url,
            )
        }
        Err(e) => {
            let page = payment_create_modal_page(&state.db, &q, form, e.to_string()).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let pay = crate::web::opt_or_log(
        PaymentEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    );
    let page = if let Some(p) = pay {
        let (posted_invoice_label, posted_invoice_href) =
            load_posted_invoice_link(&state.db, p.posted_invoice_id).await;
        let tax_ids = load_payment_tax_ids(&state.db, p.id)
            .await
            .unwrap_or_default();
        let tax_labels = invoice_header_tax_labels(&state.db, &tax_ids).await;
        let currency = load_journal_entry_currency_format(&state.db, p.journal_entry_id).await;
        PaymentDetailPage {
            id: p.id,
            posted_invoice_label,
            posted_invoice_href,
            amount: currency.display(p.amount),
            tax_labels,
            datetime: ctx.format_datetime_short(p.datetime).into_string(),
            journal_entry_id: p.journal_entry_id,
            payment_batch_id: p.payment_batch_id,
            payment_batch_href: p
                .payment_batch_id
                .map(|bid| PaymentBatchDetailRouteTag::new(bid).url()),
            can_edit: require_superuser(&ctx),
        }
    } else {
        PaymentDetailPage {
            id,
            posted_invoice_label: String::new(),
            posted_invoice_href: None,
            amount: String::new(),
            tax_labels: String::new(),
            datetime: String::new(),
            journal_entry_id: 0,
            payment_batch_id: None,
            payment_batch_href: None,
            can_edit: false,
        }
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn posted_fk_select(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ListQuery>,
) -> maud::Markup {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = PostedInvoiceEntity::find()
        .filter(crate::plugins::finance_invoices::scope::sql_posted_not_fully_paid())
        .filter(sql_posted_not_cancelled());
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => query.order_by_desc(posted_invoice::Column::Id),
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(posted_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(posted_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(posted_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(posted_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(posted_invoice::Column::Datetime)
        }
        _ => query.order_by_desc(posted_invoice::Column::Id),
    };
    let paginator = query.paginate(&state.db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows: Vec<PostedInvoiceSelectRow> = models
        .into_iter()
        .map(|p| PostedInvoiceSelectRow {
            id: p.id,
            number: p.number.clone(),
            datetime: format_invoice_date(p.datetime, &ctx.timezone),
        })
        .collect();
    let invoices = ObjectList::from_page(rows, page_num, q.page_size.get(), total);
    let page = PostedInvoiceSelectPage {
        invoices,
        target_input: q.target_input.unwrap_or_else(|| "PostedInvoiceID".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<PostedInvoiceSelectTableKey, PostedInvoiceSelectModalKey, _>(
        &htmx, &page,
    )
}
