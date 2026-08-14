use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::EntityTrait;

use crate::{
    components::{ManyToManyItem, SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done,
        respond_edit_modal_done,
    },
};

use crate::plugins::customer::entities::customer::Entity as CustomerEntity;
use crate::plugins::finance_common::require_superuser;
use crate::plugins::finance_taxes::scope::{load_taxes_by_ids, tax_label};

use crate::plugins::finance_invoices::{
    forms::DraftInvoiceForm,
    keys::{DraftInvoiceCreateModalKey, DraftInvoiceEditModalKey},
    logic::draft_payment_term::draft_payment_term_display_rows,
    logic::invoice_line_editor::{
        default_lines_json, draft_invoice_line_display_rows, draft_lines_form_json,
        invoice_line_editor_preview_json,
    },
    logic::tax_assoc::load_draft_invoice_tax_ids,
    logic::{
        CreateDraftInput, UpdateDraftInput, create_draft_invoice, default_payment_term_lines_json,
        delete_draft, format_invoice_date, optional_display, optional_trimmed_text,
        parse_invoice_datetime, parse_lines_json, parse_payment_term_lines_json,
        payment_term_lines_form_json, update_draft_invoice,
    },
    routes::DraftInvoiceDetailRouteTag,
    scope::{find_active_draft, hub_tab_url},
    state::InvoicesState,
    templates::{DraftInvoiceCreateModalPage, DraftInvoiceDetailPage, DraftInvoiceEditModalPage},
};

use super::ModalNameQuery;

#[derive(Debug, serde::Deserialize, Default)]
pub struct DeleteQuery {
    #[serde(default)]
    pub confirmed: Option<bool>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct DetailQuery {
    #[serde(default)]
    pub error: Option<String>,
}

fn form_to_input(form: &DraftInvoiceForm, tz: &str) -> Result<CreateDraftInput, String> {
    if form.customer_id <= 0 {
        return Err("select a customer".to_string());
    }
    let payment_term_lines = parse_payment_term_lines_json(&form.payment_term_lines_json)?;
    let lines = parse_lines_json(&form.invoice_lines_json)?;
    Ok(CreateDraftInput {
        number: Some(form.number.clone()),
        reference: optional_trimmed_text(&form.reference),
        payment_reference: optional_trimmed_text(&form.payment_reference),
        bank_account: optional_trimmed_text(&form.bank_account),
        datetime: parse_invoice_datetime(&form.datetime, tz),
        customer_id: form.customer_id,
        payment_term_lines,
        header_tax_ids: form.taxes.clone(),
        lines,
    })
}

struct DraftFormContext {
    customer_display: String,
    tax_items: Vec<ManyToManyItem>,
    invoice_lines_preview: String,
}

async fn load_draft_form_context(
    db: &sea_orm::DatabaseConnection,
    customer_id: i64,
    tax_ids: &[i64],
) -> DraftFormContext {
    let customer_display = if customer_id > 0 {
        CustomerEntity::find_by_id(customer_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|c| c.name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let tax_items = taxes
        .iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect();

    let invoice_lines_preview = invoice_line_editor_preview_json(db).await;

    DraftFormContext {
        customer_display,
        tax_items,
        invoice_lines_preview,
    }
}

async fn draft_create_modal_page(
    db: &sea_orm::DatabaseConnection,
    q: &ModalNameQuery,
    form: DraftInvoiceForm,
    error: String,
) -> DraftInvoiceCreateModalPage {
    let ctx_data = load_draft_form_context(db, form.customer_id, &form.taxes).await;
    DraftInvoiceCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        form,
        customer_display: ctx_data.customer_display,
        tax_items: ctx_data.tax_items,
        invoice_lines_preview: ctx_data.invoice_lines_preview,
        error,
    }
}

pub async fn create_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-invoices/").into_response();
    }
    let page = draft_create_modal_page(
        &state.db,
        &q,
        DraftInvoiceForm {
            number: String::new(),
            reference: String::new(),
            payment_reference: String::new(),
            bank_account: String::new(),
            datetime: format_invoice_date(Utc::now(), &ctx.timezone),
            customer_id: 0,
            payment_term_lines_json: default_payment_term_lines_json(),
            taxes: vec![],
            invoice_lines_json: default_lines_json(),
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
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<DraftInvoiceForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-invoices/").into_response();
    }
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => match create_draft_invoice(&state.db, input, &ctx.timezone).await {
            Ok(d) => respond_create_modal_done::<DraftInvoiceCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &DraftInvoiceDetailRouteTag::new(d.id).url(),
            ),
            Err(e) => {
                let page = draft_create_modal_page(&state.db, &q, form, e.to_string()).await;
                html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response()
            }
        },
        Err(e) => {
            let page = draft_create_modal_page(&state.db, &q, form, e).await;
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
    Query(query): Query<DetailQuery>,
) -> Response {
    let Some(d) = find_active_draft(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    };

    let tax_ids = load_draft_invoice_tax_ids(&state.db, d.id)
        .await
        .unwrap_or_default();
    let taxes = load_taxes_by_ids(&state.db, &tax_ids)
        .await
        .unwrap_or_default();
    let tax_labels = if taxes.is_empty() {
        "—".to_string()
    } else {
        taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ")
    };

    let customer_name = CustomerEntity::find_by_id(d.customer_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|c| c.name)
        .unwrap_or_else(|| format!("#{}", d.customer_id));

    let payment_term_rows = draft_payment_term_display_rows(&state.db, d.id, &ctx.timezone).await;
    let line_rows = draft_invoice_line_display_rows(&state.db, d.id).await;

    let page = DraftInvoiceDetailPage {
        id: d.id,
        number: d.number.unwrap_or_else(|| "—".to_string()),
        reference: optional_display(&d.reference),
        payment_reference: optional_display(&d.payment_reference),
        bank_account: optional_display(&d.bank_account),
        datetime: format_invoice_date(d.datetime, &ctx.timezone),
        customer_name,
        payment_term_rows,
        tax_labels,
        line_rows,
        can_edit: require_superuser(&ctx),
        error: query.error.filter(|e| !e.is_empty()),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(d) = find_active_draft(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    };

    let tax_ids = load_draft_invoice_tax_ids(&state.db, d.id)
        .await
        .unwrap_or_default();
    let ctx_data = load_draft_form_context(&state.db, d.customer_id, &tax_ids).await;
    let lines_json = draft_lines_form_json(&state.db, d.id).await;
    let payment_term_lines_json =
        payment_term_lines_form_json(&state.db, d.id, &ctx.timezone).await;
    let form = DraftInvoiceForm {
        number: d.number.unwrap_or_default(),
        reference: d.reference.unwrap_or_default(),
        payment_reference: d.payment_reference.unwrap_or_default(),
        bank_account: d.bank_account.unwrap_or_default(),
        datetime: format_invoice_date(d.datetime, &ctx.timezone),
        customer_id: d.customer_id,
        payment_term_lines_json,
        taxes: tax_ids,
        invoice_lines_json: lines_json,
    };

    let page = DraftInvoiceEditModalPage {
        id,
        form_name: q.form_name(),
        form,
        error: String::new(),
        customer_display: ctx_data.customer_display,
        tax_items: ctx_data.tax_items,
        invoice_lines_preview: ctx_data.invoice_lines_preview,
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<DraftInvoiceForm>,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    if !require_superuser(&ctx) {
        return Redirect::to(&format!("/finance-invoices/i/{id}/")).into_response();
    }
    let ctx_data = load_draft_form_context(&state.db, form.customer_id, &form.taxes).await;
    let render_error = |error: String, form: &DraftInvoiceForm| {
        let page = DraftInvoiceEditModalPage {
            id,
            form_name: q.form_name(),
            form: DraftInvoiceForm {
                number: form.number.clone(),
                reference: form.reference.clone(),
                payment_reference: form.payment_reference.clone(),
                bank_account: form.bank_account.clone(),
                datetime: form.datetime.clone(),
                customer_id: form.customer_id,
                payment_term_lines_json: form.payment_term_lines_json.clone(),
                taxes: form.taxes.clone(),
                invoice_lines_json: form.invoice_lines_json.clone(),
            },
            error,
            customer_display: ctx_data.customer_display.clone(),
            tax_items: ctx_data.tax_items.clone(),
            invoice_lines_preview: ctx_data.invoice_lines_preview.clone(),
        };
        html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
    };
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => {
            let update = UpdateDraftInput {
                number: input.number,
                reference: input.reference,
                payment_reference: input.payment_reference,
                bank_account: input.bank_account,
                datetime: input.datetime,
                customer_id: input.customer_id,
                payment_term_lines: input.payment_term_lines,
                header_tax_ids: input.header_tax_ids,
                lines: input.lines,
            };
            match update_draft_invoice(&state.db, id, update, &ctx.timezone).await {
                Ok(_) => respond_edit_modal_done::<DraftInvoiceEditModalKey>(
                    &htmx,
                    &DraftInvoiceDetailRouteTag::new(id).url(),
                ),
                Err(e) => render_error(e, &form),
            }
        }
        Err(e) => render_error(e, &form),
    }
}

pub async fn delete_post(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    if !require_superuser(&ctx) {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    let _ = delete_draft(&state.db, id).await;
    Redirect::to(&hub_tab_url("drafts")).into_response()
}

pub async fn post_invoice(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    if !require_superuser(&ctx) {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    match crate::plugins::finance_invoices::logic::draft_new_posted(&state.db, id, Utc::now()).await
    {
        Ok(p) => Redirect::to(&format!("/finance-invoices/posted/{}/", p.id)).into_response(),
        Err(e) => Redirect::to(
            &DraftInvoiceDetailRouteTag::new(id)
                .with_query()
                .query("error", &e)
                .build(),
        )
        .into_response(),
    }
}
