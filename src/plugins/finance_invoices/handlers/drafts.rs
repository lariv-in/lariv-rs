use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey, DEFAULT_PAGE_SIZE},
    html_form::{HtmlFormBody, UrlencodedFields},
    http::Cap,
    picker::respond_picker_select,
    plugins::users::middleware::RequireAuth,
    web::{
        html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done,
        respond_edit_modal_done, Htmx,
    },
};

use crate::plugins::customer::entities::customer::{self, Entity as CustomerEntity};
use crate::plugins::finance_common::require_superuser;
use crate::plugins::finance_taxes::scope::{load_taxes_by_ids, tax_label};

use crate::plugins::finance_invoices::{
    draft_form_addon::{
        render_draft_invoice_detail_extras, render_draft_invoice_form_extras,
        save_draft_invoice_form_extras, DraftInvoiceFormPost,
    },
    entities::draft_invoice::{self, Entity as DraftInvoiceEntity},
    forms::DraftInvoiceForm,
    keys::{
        DraftInvoiceBulkDeleteModalKey, DraftInvoiceCreateModalKey, DraftInvoiceDeleteModalKey,
        DraftInvoiceEditModalKey, DraftInvoiceSelectModalKey, DraftInvoiceSelectTableKey,
    },
    logic::draft_payment_term::draft_payment_term_display_rows,
    logic::invoice_line_editor::{
        default_lines_json, draft_invoice_line_display_rows, draft_lines_form_json,
        invoice_line_editor_preview_json,
    },
    logic::tax_assoc::load_draft_invoice_tax_ids,
    logic::{
        create_draft_invoice, default_payment_term_lines_json, delete_draft, format_delivery_date,
        format_invoice_date, optional_display, optional_trimmed_text, parse_delivery_date,
        parse_invoice_datetime, parse_lines_json, parse_payment_term_lines_json,
        payment_term_lines_form_json, update_draft_invoice, CreateDraftInput, UpdateDraftInput,
    },
    routes::DraftInvoiceDetailRouteTag,
    scope::{find_active_draft, hub_tab_url},
    state::InvoicesState,
    templates::{
        ConfirmBulkDeletePage, ConfirmDeletePage, DraftInvoiceCreateModalPage,
        DraftInvoiceDetailPage, DraftInvoiceEditModalPage, DraftInvoiceSelectPage,
        DraftInvoiceSelectRow,
    },
};

use super::ModalNameQuery;

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkIdsQuery {
    #[serde(default)]
    pub ids: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkIdsForm {
    #[serde(default)]
    pub ids: String,
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

fn bulk_ids_message(count: usize) -> String {
    if count == 1 {
        "Are you sure you want to delete the selected draft invoice?".into()
    } else {
        format!("Are you sure you want to delete {count} selected draft invoices?")
    }
}

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

#[derive(Debug, serde::Deserialize, Default)]
pub struct DraftInvoiceSelectQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
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
        delivery_date: parse_delivery_date(&form.delivery_date)?,
        customer_id: form.customer_id,
        payment_term_lines,
        header_tax_ids: form.taxes.clone(),
        lines,
    })
}

fn invoice_select_label(id: i64, number: &Option<String>) -> String {
    match number.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => n.to_string(),
        None => format!("#{id}"),
    }
}

struct DraftFormContext {
    customer_display: String,
    tax_items: Vec<ManyToManyItem>,
    invoice_lines_preview: String,
    extra_inputs: String,
}

async fn load_draft_form_context(
    db: &sea_orm::DatabaseConnection,
    customer_id: i64,
    tax_ids: &[i64],
    draft_id: Option<i64>,
    posted: Option<&UrlencodedFields>,
) -> DraftFormContext {
    let customer_display = if customer_id > 0 {
        crate::web::opt_or_log(
            CustomerEntity::find_by_id(customer_id).one(db).await,
            "find by id",
        )
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
    let extra_inputs = render_draft_invoice_form_extras(db, draft_id, posted).await;

    DraftFormContext {
        customer_display,
        tax_items,
        invoice_lines_preview,
        extra_inputs,
    }
}

async fn draft_create_modal_page(
    db: &sea_orm::DatabaseConnection,
    q: &ModalNameQuery,
    form: DraftInvoiceForm,
    error: String,
    posted: Option<&UrlencodedFields>,
) -> DraftInvoiceCreateModalPage {
    let ctx_data = load_draft_form_context(db, form.customer_id, &form.taxes, None, posted).await;
    DraftInvoiceCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        form,
        customer_display: ctx_data.customer_display,
        tax_items: ctx_data.tax_items,
        invoice_lines_preview: ctx_data.invoice_lines_preview,
        extra_inputs: ctx_data.extra_inputs,
        error,
    }
}

async fn draft_edit_modal_page(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    form_name: String,
    form: DraftInvoiceForm,
    error: String,
    posted: Option<&UrlencodedFields>,
) -> DraftInvoiceEditModalPage {
    let ctx_data =
        load_draft_form_context(db, form.customer_id, &form.taxes, Some(id), posted).await;
    DraftInvoiceEditModalPage {
        id,
        form_name,
        form,
        error,
        customer_display: ctx_data.customer_display,
        tax_items: ctx_data.tax_items,
        invoice_lines_preview: ctx_data.invoice_lines_preview,
        extra_inputs: ctx_data.extra_inputs,
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
            delivery_date: String::new(),
            customer_id: 0,
            payment_term_lines_json: default_payment_term_lines_json(),
            taxes: vec![],
            invoice_lines_json: default_lines_json(),
        },
        String::new(),
        None,
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
    DraftInvoiceFormPost { form, fields }: DraftInvoiceFormPost,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-invoices/").into_response();
    }
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => match create_draft_invoice(&state.db, input, &ctx.timezone).await {
            Ok(d) => {
                if let Err(e) = save_draft_invoice_form_extras(&state.db, d.id, &fields).await {
                    let page = draft_create_modal_page(&state.db, &q, form, e, Some(&fields)).await;
                    return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                        .into_response();
                }
                respond_create_modal_done::<DraftInvoiceCreateModalKey>(
                    &htmx,
                    &q.refresh_table(),
                    &DraftInvoiceDetailRouteTag::new(d.id).url(),
                )
            }
            Err(e) => {
                let page =
                    draft_create_modal_page(&state.db, &q, form, e.to_string(), Some(&fields))
                        .await;
                html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response()
            }
        },
        Err(e) => {
            let page = draft_create_modal_page(&state.db, &q, form, e, Some(&fields)).await;
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

    let customer_name = crate::web::opt_or_log(
        CustomerEntity::find_by_id(d.customer_id)
            .one(&state.db)
            .await,
        "find by id",
    )
    .map(|c| c.name)
    .unwrap_or_else(|| format!("#{}", d.customer_id));

    let payment_term_rows = draft_payment_term_display_rows(&state.db, d.id).await;
    let line_rows = draft_invoice_line_display_rows(&state.db, d.id).await;
    let extra_detail = render_draft_invoice_detail_extras(&state.db, d.id).await;

    let page = DraftInvoiceDetailPage {
        id: d.id,
        number: d.number.unwrap_or_else(|| "—".to_string()),
        reference: optional_display(&d.reference),
        payment_reference: optional_display(&d.payment_reference),
        bank_account: optional_display(&d.bank_account),
        datetime: format_invoice_date(d.datetime, &ctx.timezone),
        delivery_date: {
            let s = format_delivery_date(d.delivery_date);
            if s.is_empty() {
                "—".to_string()
            } else {
                s
            }
        },
        customer_id: d.customer_id,
        customer_name,
        payment_term_rows,
        tax_labels,
        extra_detail,
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
    let lines_json = draft_lines_form_json(&state.db, d.id).await;
    let payment_term_lines_json =
        payment_term_lines_form_json(&state.db, d.id).await;
    let form = DraftInvoiceForm {
        number: d.number.unwrap_or_default(),
        reference: d.reference.unwrap_or_default(),
        payment_reference: d.payment_reference.unwrap_or_default(),
        bank_account: d.bank_account.unwrap_or_default(),
        datetime: format_invoice_date(d.datetime, &ctx.timezone),
        delivery_date: format_delivery_date(d.delivery_date),
        customer_id: d.customer_id,
        payment_term_lines_json,
        taxes: tax_ids,
        invoice_lines_json: lines_json,
    };

    let page = draft_edit_modal_page(&state.db, id, q.form_name(), form, String::new(), None).await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    DraftInvoiceFormPost { form, fields }: DraftInvoiceFormPost,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    if !require_superuser(&ctx) {
        return Redirect::to(&format!("/finance-invoices/i/{id}/")).into_response();
    }
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => {
            let update = UpdateDraftInput {
                number: input.number,
                reference: input.reference,
                payment_reference: input.payment_reference,
                bank_account: input.bank_account,
                datetime: input.datetime,
                delivery_date: input.delivery_date,
                customer_id: input.customer_id,
                payment_term_lines: input.payment_term_lines,
                header_tax_ids: input.header_tax_ids,
                lines: input.lines,
            };
            match update_draft_invoice(&state.db, id, update, &ctx.timezone).await {
                Ok(_) => {
                    if let Err(e) = save_draft_invoice_form_extras(&state.db, id, &fields).await {
                        let page = draft_edit_modal_page(
                            &state.db,
                            id,
                            q.form_name(),
                            form,
                            e,
                            Some(&fields),
                        )
                        .await;
                        return html_built_page_with_slots(
                            &page,
                            &chrome,
                            &SlotCtx::from_auth(&ctx),
                        )
                        .into_response();
                    }
                    respond_edit_modal_done::<DraftInvoiceEditModalKey>(
                        &htmx,
                        &DraftInvoiceDetailRouteTag::new(id).url(),
                    )
                }
                Err(e) => {
                    let page =
                        draft_edit_modal_page(&state.db, id, q.form_name(), form, e, Some(&fields))
                            .await;
                    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                        .into_response()
                }
            }
        }
        Err(e) => {
            let page =
                draft_edit_modal_page(&state.db, id, q.form_name(), form, e, Some(&fields)).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: DraftInvoiceDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this draft invoice?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_finance_invoices.DraftInvoiceDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    if !require_superuser(&ctx) {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    match delete_draft(&state.db, id).await {
        Ok(_) => htmx.redirect(&hub_tab_url("drafts")),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete draft invoice");
            let page = ConfirmDeletePage {
                modal_uid: DraftInvoiceDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this draft invoice?".into(),
                form_name: "p_finance_invoices.DraftInvoiceDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
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
    match crate::plugins::finance_invoices::logic::draft_new_posted(
        &state.db,
        id,
        Utc::now(),
        &ctx.timezone,
    )
    .await
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

pub async fn bulk_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkIdsQuery>,
) -> maud::Markup {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let page = ConfirmBulkDeletePage {
        modal_uid: DraftInvoiceBulkDeleteModalKey::ID.to_string(),
        message: if ids.is_empty() {
            "Select at least one draft invoice to delete.".into()
        } else {
            bulk_ids_message(ids.len())
        },
        form_name: "p_finance_invoices.DraftInvoiceBulkDeleteForm".into(),
        ids: ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
        error: if ids.is_empty() {
            "No invoices selected.".into()
        } else {
            String::new()
        },
        can_submit: !ids.is_empty(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn bulk_delete_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<BulkIdsForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    let ids = parse_bulk_ids(&form.ids);
    if ids.is_empty() {
        let page = ConfirmBulkDeletePage {
            modal_uid: DraftInvoiceBulkDeleteModalKey::ID.to_string(),
            message: "Select at least one draft invoice to delete.".into(),
            form_name: "p_finance_invoices.DraftInvoiceBulkDeleteForm".into(),
            ids: String::new(),
            error: "No invoices selected.".into(),
            can_submit: false,
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response();
    }
    for id in &ids {
        if find_active_draft(&state.db, *id).await.is_none() {
            continue;
        }
        if let Err(e) = delete_draft(&state.db, *id).await {
            tracing::error!(error = %e, id, "failed to bulk-delete draft invoice");
            let page = ConfirmBulkDeletePage {
                modal_uid: DraftInvoiceBulkDeleteModalKey::ID.to_string(),
                message: bulk_ids_message(ids.len()),
                form_name: "p_finance_invoices.DraftInvoiceBulkDeleteForm".into(),
                ids: ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                error: format!("Failed to delete draft #{id}: {e}"),
                can_submit: true,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    }
    htmx.redirect(&hub_tab_url("drafts"))
}

pub async fn bulk_post(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkIdsQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    if ids.is_empty() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    let now = Utc::now();
    for id in ids {
        if find_active_draft(&state.db, id).await.is_none() {
            continue;
        }
        if let Err(e) = crate::plugins::finance_invoices::logic::draft_new_posted(
            &state.db,
            id,
            now,
            &ctx.timezone,
        )
        .await
        {
            return Redirect::to(
                &DraftInvoiceDetailRouteTag::new(id)
                    .with_query()
                    .query("error", &e)
                    .build(),
            )
            .into_response();
        }
    }
    Redirect::to(&hub_tab_url("posted")).into_response()
}

pub async fn multi_select(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<DraftInvoiceSelectQuery>,
) -> maud::Markup {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = DraftInvoiceEntity::find();
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => {
            query.order_by_desc(draft_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(draft_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(draft_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(draft_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(draft_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(draft_invoice::Column::Datetime)
        }
        _ => query.order_by_desc(draft_invoice::Column::Datetime),
    };
    let paginator = query.paginate(&state.db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let customer_ids: Vec<i64> = models.iter().map(|d| d.customer_id).collect();
    let customers = if customer_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        CustomerEntity::find()
            .filter(customer::Column::Id.is_in(customer_ids))
            .all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect()
    };
    let rows: Vec<DraftInvoiceSelectRow> = models
        .into_iter()
        .map(|d| DraftInvoiceSelectRow {
            id: d.id,
            number: invoice_select_label(d.id, &d.number),
            datetime: format_invoice_date(d.datetime, &ctx.timezone),
            customer_name: customers
                .get(&d.customer_id)
                .cloned()
                .unwrap_or_else(|| format!("#{}", d.customer_id)),
        })
        .collect();
    let invoices = ObjectList::from_page(rows, page_num, PAGE_SIZE, total);
    let page = DraftInvoiceSelectPage {
        invoices,
        target_input: q.target_input.unwrap_or_else(|| "Invoices".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    respond_picker_select::<DraftInvoiceSelectTableKey, DraftInvoiceSelectModalKey, _>(&htmx, &page)
}
