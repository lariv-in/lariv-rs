use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    template::RenderAppPane,
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done,
    },
};

use crate::plugins::crm::{
    deal_stage::DealStage,
    entities::{
        converted_lead::{self, Entity as ConvertedLeadEntity},
        failed_lead::{self, Entity as FailedLeadEntity},
        lead::{self, Entity as LeadEntity},
    },
    forms::{ConvertLeadBody, FailLeadForm, LeadEditBody, LeadForm},
    handlers::ModalNameQuery,
    keys::{LeadConvertModalKey, LeadCreateModalKey, LeadFailModalKey, LeadHubTableKey},
    lead_source::LeadSource,
    logic::{
        lead::{LeadInput, create_lead, delete_lead, update_lead},
        lead_conversion::{ConvertLeadDeal, ConvertLeadInput, convert_lead},
        lead_fail::{fail_lead, reactivate_lead, update_failed_reason},
    },
    routes::{
        ConvertedLeadDetailRouteTag, FailedLeadDetailRouteTag, LeadDetailRouteTag,
    },
    scope::{
        apply_lead_filters, company_display_label, find_active_lead, find_company_scoped,
        find_converted_lead_scoped, find_failed_lead_scoped, find_lead_scoped, sql_lead_active,
    },
    state::CrmState,
    templates::{
        ConvertLeadModalPage, FailLeadModalPage, LeadConvertDetailPage, LeadCreateModalPage,
        LeadDetailPage, LeadFailDetailPage, LeadFormPage, LeadHubPage, LeadRow,
    },
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct HubQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default, rename = "Company", alias = "company")]
    pub company: Option<String>,
    #[serde(default, rename = "Email", alias = "email")]
    pub email: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn opt_string(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn parse_lead_source(raw: &str) -> LeadSource {
    LeadSource::parse(raw).unwrap_or_default()
}

fn parse_deal_stage(raw: &str) -> DealStage {
    DealStage::parse(raw).unwrap_or_default()
}

fn convert_input_from_body(form: &ConvertLeadBody) -> ConvertLeadInput {
    let deal = if form.deal_kind == "Create" {
        let amount = if form.deal_amount.trim().is_empty() {
            None
        } else {
            form.deal_amount.parse::<rust_decimal::Decimal>().ok()
        };
        ConvertLeadDeal::Create {
            deal_name: opt_string(form.deal_name.clone()),
            deal_amount: amount,
            deal_stage: parse_deal_stage(&form.deal_stage),
        }
    } else {
        ConvertLeadDeal::None
    };
    ConvertLeadInput {
        company_id: form.company_id,
        deal,
    }
}

async fn convert_modal_page_from_body(
    db: &sea_orm::DatabaseConnection,
    lead_id: i64,
    q: &ModalNameQuery,
    form: &ConvertLeadBody,
    error: String,
) -> ConvertLeadModalPage {
    ConvertLeadModalPage {
        lead_id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        company_id: form.company_id,
        company_display: company_display_label(db, form.company_id).await,
        deal_kind: form.deal_kind.clone(),
        deal_name: form.deal_name.clone(),
        deal_amount: form.deal_amount.clone(),
        deal_stage: if form.deal_stage.is_empty() {
            DealStage::default().as_str().to_string()
        } else {
            form.deal_stage.clone()
        },
        error,
    }
}


fn lead_input_from_form(form: &LeadForm) -> LeadInput {
    LeadInput {
        company_name: opt_string(form.company_name.clone()),
        first_name: opt_string(form.first_name.clone()),
        last_name: opt_string(form.last_name.clone()),
        email: opt_string(form.email.clone()),
        phone: opt_string(form.phone.clone()),
        source: parse_lead_source(&form.source),
        notes: opt_string(form.notes.clone()),
    }
}

async fn lead_return_url(
    db: &sea_orm::DatabaseConnection,
    lead_id: i64,
) -> String {
    if let Some(c) = ConvertedLeadEntity::find()
        .filter(converted_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .ok()
        .flatten()
    {
        return ConvertedLeadDetailRouteTag::new(c.id).url();
    }
    if let Some(f) = FailedLeadEntity::find()
        .filter(failed_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .ok()
        .flatten()
    {
        return FailedLeadDetailRouteTag::new(f.id).url();
    }
    LeadDetailRouteTag::new(lead_id).url()
}

async fn lead_sidebar_context(
    db: &sea_orm::DatabaseConnection,
    lead_id: i64,
    display_name: &str,
) -> (String, String, String) {
    if ConvertedLeadEntity::find()
        .filter(converted_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return (
            format!("Converted lead: {display_name}"),
            lead_return_url(db, lead_id).await,
            "converted".to_string(),
        );
    }
    if FailedLeadEntity::find()
        .filter(failed_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return (
            format!("Failed lead: {display_name}"),
            lead_return_url(db, lead_id).await,
            "failed".to_string(),
        );
    }
    (
        format!("Lead: {display_name}"),
        LeadDetailRouteTag::new(lead_id).url(),
        "active".to_string(),
    )
}

async fn query_active_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = LeadEntity::find().filter(sql_lead_active());
    query = apply_lead_filters(query, q.company.as_deref(), q.email.as_deref());
    query = query.order_by_desc(lead::Column::CreatedAt);
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|l| LeadRow {
            id: l.id,
            name: l.display_name(),
            company: l.company_name.unwrap_or_default(),
            email: l.email.unwrap_or_default(),
            source: l.source.label().to_string(),
            status: "Active".to_string(),
            detail_href: LeadDetailRouteTag::new(l.id).url(),
        })
        .collect();
    (rows, page_num, total)
}

async fn query_converted_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let query = ConvertedLeadEntity::find().order_by_desc(converted_lead::Column::ConvertedAt);
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let converted = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let company_filter = q.company.as_deref().filter(|s| !s.is_empty());
    let email_filter = q.email.as_deref().filter(|s| !s.is_empty());
    let mut rows = Vec::new();
    for c in converted {
        let lead = LeadEntity::find_by_id(c.lead_id)
            .one(db)
            .await
            .ok()
            .flatten();
        let (name, company, email, source) = match lead {
            Some(l) => (
                l.display_name(),
                l.company_name.clone().unwrap_or_default(),
                l.email.clone().unwrap_or_default(),
                l.source.label().to_string(),
            ),
            None => (
                format!("Lead #{}", c.lead_id),
                String::new(),
                String::new(),
                String::new(),
            ),
        };
        if let Some(cf) = company_filter {
            if !company.contains(cf) {
                continue;
            }
        }
        if let Some(ef) = email_filter {
            if !email.contains(ef) {
                continue;
            }
        }
        rows.push(LeadRow {
            id: c.id,
            name,
            company,
            email,
            source,
            status: "Converted".to_string(),
            detail_href: ConvertedLeadDetailRouteTag::new(c.id).url(),
        });
    }
    (rows, page_num, total)
}

async fn query_failed_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = FailedLeadEntity::find();
    query = query.order_by_desc(failed_lead::Column::FailedAt);
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let failed = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::new();
    for f in failed {
        let lead = LeadEntity::find_by_id(f.lead_id)
            .one(db)
            .await
            .ok()
            .flatten();
        let (name, company, email, source) = match lead {
            Some(l) => (
                l.display_name(),
                l.company_name.unwrap_or_default(),
                l.email.unwrap_or_default(),
                l.source.label().to_string(),
            ),
            None => (format!("Lead #{}", f.lead_id), String::new(), String::new(), String::new()),
        };
        rows.push(LeadRow {
            id: f.id,
            name,
            company,
            email,
            source,
            status: "Failed".to_string(),
            detail_href: FailedLeadDetailRouteTag::new(f.id).url(),
        });
    }
    (rows, page_num, total)
}

pub async fn hub(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<HubQuery>,
) -> maud::Markup {
    let tab = q.tab.as_deref().unwrap_or("active").to_string();
    let (rows, page, total) = match tab.as_str() {
        "converted" => query_converted_leads(&state.db, &q, PAGE_SIZE).await,
        "failed" => query_failed_leads(&state.db, &q, PAGE_SIZE).await,
        _ => query_active_leads(&state.db, &q, PAGE_SIZE).await,
    };
    let leads = ObjectList::from_page(rows, page, PAGE_SIZE, total);
    let page = LeadHubPage {
        leads,
        tab,
        filter_company: q.company.clone().unwrap_or_default(),
        filter_email: q.email.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<LeadHubTableKey>() {
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
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = LeadCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        company_name: String::new(),
        first_name: String::new(),
        last_name: String::new(),
        email: String::new(),
        phone: String::new(),
        source: LeadSource::default().as_str().to_string(),
        notes: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<LeadForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    match create_lead(&state.db, lead_input_from_form(&form)).await {
        Ok(saved) => respond_create_modal_done::<LeadCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &LeadDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = LeadCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                company_name: form.company_name,
                first_name: form.first_name,
                last_name: form.last_name,
                email: form.email,
                phone: form.phone,
                source: form.source,
                notes: form.notes,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(lead) = find_lead_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    if find_active_lead(&state.db, id, &ctx).await.is_none() {
        if let Some(c) = ConvertedLeadEntity::find()
            .filter(converted_lead::Column::LeadId.eq(id))
            .one(&state.db)
            .await
            .ok()
            .flatten()
        {
            return Redirect::to(&ConvertedLeadDetailRouteTag::new(c.id).url()).into_response();
        }
        if let Some(f) = FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(id))
            .one(&state.db)
            .await
            .ok()
            .flatten()
        {
            return Redirect::to(&FailedLeadDetailRouteTag::new(f.id).url()).into_response();
        }
        return Redirect::to("/crm/leads").into_response();
    }
    let page = LeadDetailPage {
        id: lead.id,
        display_name: lead.display_name(),
        company_name: lead.company_name.unwrap_or_default(),
        first_name: lead.first_name.unwrap_or_default(),
        last_name: lead.last_name.unwrap_or_default(),
        email: lead.email.unwrap_or_default(),
        phone: lead.phone.unwrap_or_default(),
        source: lead.source.label().to_string(),
        notes: lead.notes.unwrap_or_default(),
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    let Some(lead) = find_lead_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let failed = FailedLeadEntity::find()
        .filter(failed_lead::Column::LeadId.eq(id))
        .one(&state.db)
        .await
        .ok()
        .flatten();
    let display_name = lead.display_name();
    let (menu_title, detail_url, list_tab) =
        lead_sidebar_context(&state.db, id, &display_name).await;
    let page = LeadFormPage {
        id: lead.id,
        company_name: lead.company_name.unwrap_or_default(),
        first_name: lead.first_name.unwrap_or_default(),
        last_name: lead.last_name.unwrap_or_default(),
        email: lead.email.unwrap_or_default(),
        phone: lead.phone.unwrap_or_default(),
        source: lead.source.as_str().to_string(),
        notes: lead.notes.unwrap_or_default(),
        reason: failed
            .as_ref()
            .and_then(|f| f.reason.clone())
            .unwrap_or_default(),
        show_reason: failed.is_some(),
        menu_title,
        detail_url,
        display_name,
        list_tab,
        can_edit: true,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Form(form): Form<LeadEditBody>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    if find_lead_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to("/crm/leads").into_response();
    }
    if update_lead(&state.db, id, lead_input_from_form(&form.lead))
        .await
        .is_err()
    {
        return Redirect::to("/crm/leads").into_response();
    }
    if FailedLeadEntity::find()
        .filter(failed_lead::Column::LeadId.eq(id))
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .is_some()
        && update_failed_reason(&state.db, id, &ctx, opt_string(form.reason.clone()))
            .await
            .is_err()
    {
        return Redirect::to("/crm/leads").into_response();
    }
    Redirect::to(&lead_return_url(&state.db, id).await).into_response()
}

pub async fn delete_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    let _ = delete_lead(&state.db, id).await;
    Redirect::to("/crm/leads").into_response()
}

pub async fn convert_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    if find_active_lead(&state.db, id, &ctx).await.is_none() {
        return Redirect::to("/crm/leads").into_response();
    }
    let page = ConvertLeadModalPage {
        lead_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        company_id: 0,
        company_display: String::new(),
        deal_kind: "None".to_string(),
        deal_name: String::new(),
        deal_amount: String::new(),
        deal_stage: DealStage::default().as_str().to_string(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn convert_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<ConvertLeadBody>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    if form.company_id <= 0
        || find_company_scoped(&state.db, form.company_id, &ctx)
            .await
            .is_none()
    {
        let page = convert_modal_page_from_body(
            &state.db,
            id,
            &q,
            &form,
            "company is required".to_string(),
        )
        .await;
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response();
    }
    let input = convert_input_from_body(&form);
    match convert_lead(&state.db, id, &ctx, input).await {
        Ok(result) => respond_create_modal_done::<LeadConvertModalKey>(
            &htmx,
            &q.refresh_table(),
            &ConvertedLeadDetailRouteTag::new(result.converted_id).url(),
        ),
        Err(e) => {
            let page = convert_modal_page_from_body(&state.db, id, &q, &form, e).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn fail_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    if find_active_lead(&state.db, id, &ctx).await.is_none() {
        return Redirect::to("/crm/leads").into_response();
    }
    let page = FailLeadModalPage {
        lead_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        reason: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn fail_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<FailLeadForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    match fail_lead(&state.db, id, &ctx, opt_string(form.reason.clone())).await {
        Ok(failed_id) => respond_create_modal_done::<LeadFailModalKey>(
            &htmx,
            &q.refresh_table(),
            &FailedLeadDetailRouteTag::new(failed_id).url(),
        ),
        Err(e) => {
            let page = FailLeadModalPage {
                lead_id: id,
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                reason: form.reason,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn converted_detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(converted) = find_converted_lead_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let lead = find_lead_scoped(&state.db, converted.lead_id, &ctx).await;
    let display_name = lead
        .as_ref()
        .map(|l| l.display_name())
        .unwrap_or_else(|| format!("Lead #{}", converted.lead_id));
    let page = LeadConvertDetailPage {
        converted_id: converted.id,
        lead_id: converted.lead_id,
        display_name,
        converted_at: converted.converted_at.format("%Y-%m-%d %H:%M UTC").to_string(),
        company_id: converted.company_id,
        contact_id: converted.contact_id,
        customer_id: converted.customer_id,
        deal_id: converted.deal_id,
        company_name: lead
            .as_ref()
            .and_then(|l| l.company_name.clone())
            .unwrap_or_default(),
        first_name: lead
            .as_ref()
            .and_then(|l| l.first_name.clone())
            .unwrap_or_default(),
        last_name: lead
            .as_ref()
            .and_then(|l| l.last_name.clone())
            .unwrap_or_default(),
        email: lead
            .as_ref()
            .and_then(|l| l.email.clone())
            .unwrap_or_default(),
        phone: lead
            .as_ref()
            .and_then(|l| l.phone.clone())
            .unwrap_or_default(),
        source: lead
            .as_ref()
            .map(|l| l.source.label().to_string())
            .unwrap_or_default(),
        notes: lead
            .as_ref()
            .and_then(|l| l.notes.clone())
            .unwrap_or_default(),
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn failed_detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(failed) = find_failed_lead_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let lead = find_lead_scoped(&state.db, failed.lead_id, &ctx).await;
    let display_name = lead
        .as_ref()
        .map(|l| l.display_name())
        .unwrap_or_else(|| format!("Lead #{}", failed.lead_id));
    let page = LeadFailDetailPage {
        failed_id: failed.id,
        lead_id: failed.lead_id,
        display_name,
        failed_at: failed.failed_at.format("%Y-%m-%d %H:%M UTC").to_string(),
        reason: failed.reason.unwrap_or_default(),
        company_name: lead
            .as_ref()
            .and_then(|l| l.company_name.clone())
            .unwrap_or_default(),
        first_name: lead
            .as_ref()
            .and_then(|l| l.first_name.clone())
            .unwrap_or_default(),
        last_name: lead
            .as_ref()
            .and_then(|l| l.last_name.clone())
            .unwrap_or_default(),
        email: lead
            .as_ref()
            .and_then(|l| l.email.clone())
            .unwrap_or_default(),
        phone: lead
            .as_ref()
            .and_then(|l| l.phone.clone())
            .unwrap_or_default(),
        source: lead
            .as_ref()
            .map(|l| l.source.label().to_string())
            .unwrap_or_default(),
        notes: lead
            .as_ref()
            .and_then(|l| l.notes.clone())
            .unwrap_or_default(),
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn reactivate_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    match reactivate_lead(&state.db, id, &ctx).await {
        Ok(lead_id) => Redirect::to(&LeadDetailRouteTag::new(lead_id).url()).into_response(),
        Err(_) => Redirect::to("/crm/leads").into_response(),
    }
}
