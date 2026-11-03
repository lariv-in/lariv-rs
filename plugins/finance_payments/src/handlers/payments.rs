use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use lariv_core::datetime::DatetimeLabel;
use maud::{Markup, html};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use lariv_core::components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{CsrfToken, HtmlFormBody};
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done,
};
use lariv_plugin_customer::entities::customer::Entity as CustomerEntity;
use lariv_plugin_finance_accounts::journal_lines_editor::input_journal_lines;
use lariv_plugin_finance_accounts::logic::custom_source_doc::{
    ensure_posting_accounts, parse_submitted_journal_lines,
};
use lariv_plugin_finance_accounts::scope::{
    CurrencyFormat, load_journal_entry_currency_format, load_journal_entry_currency_formats,
};
use lariv_plugin_finance_taxes::scope::{load_taxes_by_ids, tax_label};
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    entities::payment::{self, Entity as PaymentEntity},
    forms::PaymentForm,
    handlers::ModalNameQuery,
    keys::{CUSTOMER_PAYMENTS_TABLE, PAYMENT_CREATE_MODAL, PAYMENT_TABLE},
    logic::{
        CreatePaymentInput, create_payment, parse_payment_amount, suggest_payment_lines_json,
        tax_assoc::load_payment_tax_ids,
    },
    routes::PaymentDetailRouteTag,
    state::PaymentsState,
    templates::{PaymentCreateModalPage, PaymentDetailPage, PaymentListPage, PaymentRow},
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct ListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct PaymentCreateQuery {
    #[serde(flatten)]
    pub modal: ModalNameQuery,
    #[serde(default, rename = "CustomerID")]
    pub customer_id: Option<i64>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn parse_payment_datetime(s: &str, tz: &str) -> chrono::DateTime<Utc> {
    let s = s.trim();
    if s.is_empty() {
        return Utc::now();
    }
    if let Some(dt) = lariv_core::datetime::parse_date_start_in_tz(s, tz) {
        return dt;
    }
    lariv_core::datetime::DatetimeLocalInput::from_raw(s)
        .to_stored(tz)
        .unwrap_or_else(Utc::now)
}

async fn customer_label(db: &sea_orm::DatabaseConnection, customer_id: i64) -> String {
    if customer_id <= 0 {
        return "—".into();
    }
    let name = lariv_plugin_customer::logic::display_name(db, customer_id).await;
    if name.is_empty() {
        format!("#{customer_id}")
    } else {
        name
    }
}

async fn customer_href(db: &sea_orm::DatabaseConnection, customer_id: i64) -> Option<String> {
    let row = CustomerEntity::find_by_id(customer_id)
        .one(db)
        .await
        .ok()??;
    Some(lariv_plugin_customer::logic::detail_href(&row))
}

async fn tax_items(db: &sea_orm::DatabaseConnection, tax_ids: &[i64]) -> Vec<ManyToManyItem> {
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    taxes
        .iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect()
}

async fn load_payment_page(
    db: &sea_orm::DatabaseConnection,
    q: &ListQuery,
    timezone: &str,
    path_and_query: String,
    customer_id: Option<i64>,
    table_key: lariv_core::components::SwapKey,
    filter_url: String,
) -> PaymentListPage {
    let page_num = q.page.get();
    let page_size = q.page_size.get();
    let mut query = PaymentEntity::find();
    if let Some(id) = customer_id {
        query = query.filter(payment::Column::CustomerId.eq(id));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
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
    let je_ids: Vec<i64> = models.iter().map(|p| p.journal_entry_id).collect();
    let currency_fmts = load_journal_entry_currency_formats(db, &je_ids).await;
    let fallback = CurrencyFormat::fallback();
    let mut rows = Vec::with_capacity(models.len());
    for p in models {
        let fmt = currency_fmts.get(&p.journal_entry_id).unwrap_or(&fallback);
        rows.push(PaymentRow {
            id: p.id,
            customer_label: customer_label(db, p.customer_id).await,
            amount: fmt.display(p.amount),
            datetime: DatetimeLabel::display(p.datetime, timezone).into_string(),
        });
    }
    PaymentListPage {
        payments: ObjectList::from_page(rows, page_num, page_size, total),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query,
        can_edit: lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                crate::routes::FinancePaymentsMutate,
            >(),
        ),
        page_size,
        table_key,
        filter_url,
        create_customer_id: customer_id.filter(|id| *id > 0),
    }
}

/// Payments table constrained to one customer, with the main list's columns and page-size filter.
pub struct CustomerPaymentsTable<'a> {
    pub customer_id: i64,
    pub path_and_query: &'a str,
    pub filter_url: &'a str,
    pub timezone: &'a str,
    pub q: &'a ListQuery,
}

pub async fn customer_payments_table(
    db: &sea_orm::DatabaseConnection,
    spec: CustomerPaymentsTable<'_>,
) -> maud::Markup {
    let page = load_payment_page(
        db,
        spec.q,
        spec.timezone,
        spec.path_and_query.to_string(),
        Some(spec.customer_id),
        CUSTOMER_PAYMENTS_TABLE,
        spec.filter_url.to_string(),
    )
    .await;
    page.render_table()
}

pub async fn list(
    Cap(state): Cap<PaymentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ListQuery>,
) -> maud::Markup {
    let page = load_payment_page(
        &state.db,
        &q,
        &ctx.timezone,
        path_and_query(&uri),
        None,
        PAYMENT_TABLE,
        String::new(),
    )
    .await;
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(PAYMENT_TABLE) {
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
    Cap(state): Cap<PaymentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<PaymentCreateQuery>,
) -> Response {
    let customer_id = q.customer_id.filter(|id| *id > 0).unwrap_or(0);
    let customer_display = customer_label(&state.db, customer_id).await;
    let form = PaymentForm {
        customer_id,
        amount: String::new(),
        datetime: ctx.datetime_local_input(Utc::now()).into_string(),
        taxes: vec![],
        journal_lines: String::new(),
        csrf: CsrfToken::current(),
    };
    let page = PaymentCreateModalPage {
        form_name: q.modal.form_name(),
        refresh_table: q.modal.refresh_table(),
        form,
        customer_display,
        tax_items: Vec::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn create_modal(
    db: &sea_orm::DatabaseConnection,
    q: &PaymentCreateQuery,
    form: PaymentForm,
    error: String,
) -> PaymentCreateModalPage {
    let customer_display = customer_label(db, form.customer_id).await;
    let tax_items = tax_items(db, &form.taxes).await;
    PaymentCreateModalPage {
        form_name: q.modal.form_name(),
        refresh_table: q.modal.refresh_table(),
        form,
        customer_display,
        tax_items,
        error,
    }
}

pub async fn create_post(
    Cap(state): Cap<PaymentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<PaymentCreateQuery>,
    HtmlFormBody(form): HtmlFormBody<PaymentForm>,
) -> Response {
    let amount = match parse_payment_amount(&form.amount) {
        Ok(a) => a,
        Err(e) => {
            let page = create_modal(&state.db, &q, form, e).await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let submitted = match parse_submitted_journal_lines(&form.journal_lines) {
        Ok(lines) => lines,
        Err(error) => {
            let page = create_modal(&state.db, &q, form, error).await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    if let Err(error) = ensure_posting_accounts(&state.db, &submitted.lines).await {
        let page = create_modal(&state.db, &q, form, error).await;
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let input = CreatePaymentInput {
        customer_id: form.customer_id,
        amount,
        datetime: parse_payment_datetime(&form.datetime, &ctx.timezone),
        withholding_tax_ids: form.taxes.clone(),
        journal_lines: Some(submitted.lines),
        withholding_base: None,
    };
    match create_payment(&state.db, input).await {
        Ok(payment) => respond_create_modal_done(
            &htmx,
            PAYMENT_CREATE_MODAL,
            &q.modal.refresh_table(),
            &PaymentDetailRouteTag::new(payment.id).url(),
        ),
        Err(e) => {
            let page = create_modal(&state.db, &q, form, e).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn fill_journal_lines(
    Cap(state): Cap<PaymentsState>,
    RequireAuth(_ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<PaymentForm>,
) -> Markup {
    let suggested = match parse_payment_amount(&form.amount) {
        Ok(amount) => suggest_payment_lines_json(&state.db, amount, amount, &form.taxes).await,
        Err(error) => Err(error),
    };
    match suggested {
        Ok(lines) => input_journal_lines("JournalLines", "Journal lines", &lines),
        Err(error) => html! {
            p class="text-error mb-2" { (error) }
            (input_journal_lines("JournalLines", "Journal lines", &form.journal_lines))
        },
    }
}

pub async fn detail(
    Cap(state): Cap<PaymentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let pay = lariv_core::web::opt_or_log(
        PaymentEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    );
    let page = if let Some(p) = pay {
        let tax_ids = load_payment_tax_ids(&state.db, p.id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(&state.db, &tax_ids)
            .await
            .unwrap_or_default();
        let tax_labels = taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ");
        let currency = load_journal_entry_currency_format(&state.db, p.journal_entry_id).await;
        PaymentDetailPage {
            id: p.id,
            customer_label: customer_label(&state.db, p.customer_id).await,
            customer_href: customer_href(&state.db, p.customer_id).await,
            amount: currency.display(p.amount),
            tax_labels,
            datetime: ctx.format_datetime(p.datetime).into_string(),
            journal_entry_id: p.journal_entry_id,
        }
    } else {
        PaymentDetailPage {
            id,
            customer_label: String::new(),
            customer_href: None,
            amount: String::new(),
            tax_labels: String::new(),
            datetime: String::new(),
            journal_entry_id: 0,
        }
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}
