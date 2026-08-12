use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryOrder};

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done,
    },
};

use crate::plugins::crm::{
    deal_stage::DealStage,
    entities::deal::{self, Entity as DealEntity},
    forms::DealForm,
    handlers::ModalNameQuery,
    keys::{DealCreateModalKey, DealTableKey},
    routes::DealDetailRouteTag,
    scope::{
        apply_deal_filters, company_display_label, contact_belongs_to_company,
        contact_display_label, find_company_scoped, find_contact_scoped, find_deal_scoped,
        scope_superuser,
    },
    state::CrmState,
    templates::{DealCreateModalPage, DealDetailPage, DealFormPage, DealListPage, DealRow},
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct DealListQuery {
    #[serde(default, rename = "CompanyId", alias = "company_id")]
    pub company_id: Option<String>,
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn parse_i64(raw: &str) -> Option<i64> {
    raw.trim().parse().ok().filter(|id| *id > 0)
}

fn parse_deal_stage(raw: &str) -> DealStage {
    DealStage::parse(raw).unwrap_or_default()
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

async fn filter_company_display(
    db: &sea_orm::DatabaseConnection,
    company_id: Option<&str>,
) -> String {
    let Some(id) = company_id.and_then(parse_i64) else {
        return String::new();
    };
    company_display_label(db, id).await
}

async fn query_deals(
    db: &sea_orm::DatabaseConnection,
    q: &DealListQuery,
    auth: &crate::plugins::users::state::AuthContext,
    page_size: u32,
) -> (Vec<deal::Model>, u32, u64) {
    let company_id = q.company_id.as_deref().and_then(parse_i64);
    let mut query = DealEntity::find();
    query = apply_deal_filters(query, company_id, q.name.as_deref());
    query = scope_superuser(query, auth);
    query = query.order_by_desc(deal::Column::CreatedAt);
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

fn model_to_row(d: deal::Model) -> DealRow {
    DealRow {
        id: d.id,
        company_id: d.company_id,
        name: d.name,
        stage: d.stage.label().to_string(),
        amount: d
            .amount
            .map(|a| a.to_string())
            .unwrap_or_default(),
    }
}

async fn load_deal_rows(
    db: &sea_orm::DatabaseConnection,
    q: &DealListQuery,
    auth: &crate::plugins::users::state::AuthContext,
    page_size: u32,
) -> ObjectList<DealRow> {
    let (models, page, total) = query_deals(db, q, auth, page_size).await;
    let rows = models.into_iter().map(model_to_row).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<DealListQuery>,
) -> maud::Markup {
    let deals = load_deal_rows(&state.db, &q, &ctx, PAGE_SIZE).await;
    let page = DealListPage {
        deals,
        filter_company_id: q.company_id.clone().unwrap_or_default(),
        filter_company_display: filter_company_display(&state.db, q.company_id.as_deref()).await,
        filter_name: q.name.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<DealTableKey>() {
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

pub async fn detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(deal) = find_deal_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/deals").into_response();
    };
    let page = DealDetailPage {
        id: deal.id,
        company_id: deal.company_id,
        primary_contact_id: deal.primary_contact_id,
        name: deal.name,
        amount: deal.amount.map(|a| a.to_string()).unwrap_or_default(),
        stage: deal.stage.label().to_string(),
        expected_close_date: deal
            .expected_close_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = DealCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        company_id: 0,
        company_display: String::new(),
        primary_contact_id: 0,
        primary_contact_display: String::new(),
        name: String::new(),
        amount: String::new(),
        stage: DealStage::default().as_str().to_string(),
        expected_close_date: String::new(),
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
    Form(form): Form<DealForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/deals").into_response();
    }
    let company_id = form.company_id;
    let primary_contact_id = form.primary_contact_id;
    if company_id <= 0 {
        return validation_error(
            &state.db,
            &chrome,
            &ctx,
            &q,
            &form,
            "company is required",
        )
        .await;
    }
    if primary_contact_id <= 0 {
        return validation_error(
            &state.db,
            &chrome,
            &ctx,
            &q,
            &form,
            "primary contact is required",
        )
        .await;
    }
    if find_company_scoped(&state.db, company_id, &ctx).await.is_none() {
        return Redirect::to("/crm/deals").into_response();
    }
    if find_contact_scoped(&state.db, primary_contact_id, &ctx).await.is_none() {
        return Redirect::to("/crm/deals").into_response();
    }
    if !contact_belongs_to_company(&state.db, primary_contact_id, company_id).await {
        return validation_error(
            &state.db,
            &chrome,
            &ctx,
            &q,
            &form,
            "primary contact must belong to the company",
        )
        .await;
    }
    let now = Utc::now();
    let amount = if form.amount.trim().is_empty() {
        None
    } else {
        form.amount.parse::<rust_decimal::Decimal>().ok()
    };
    let model = deal::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        company_id: Set(company_id),
        primary_contact_id: Set(primary_contact_id),
        name: Set(form.name.clone()),
        amount: Set(amount),
        stage: Set(parse_deal_stage(&form.stage)),
        expected_close_date: Set(parse_date(&form.expected_close_date)),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<DealCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &DealDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            validation_error(&state.db, &chrome, &ctx, &q, &form, &e.to_string()).await
        }
    }
}

async fn validation_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &crate::plugins::users::state::AuthContext,
    q: &ModalNameQuery,
    form: &DealForm,
    error: &str,
) -> Response {
    let page = DealCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        company_id: form.company_id,
        company_display: company_display_label(db, form.company_id).await,
        primary_contact_id: form.primary_contact_id,
        primary_contact_display: contact_display_label(db, form.primary_contact_id).await,
        name: form.name.clone(),
        amount: form.amount.clone(),
        stage: form.stage.clone(),
        expected_close_date: form.expected_close_date.clone(),
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/deals").into_response();
    }
    let Some(deal) = find_deal_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/deals").into_response();
    };
    let page = DealFormPage {
        id: deal.id,
        company_id: deal.company_id,
        company_display: company_display_label(&state.db, deal.company_id).await,
        primary_contact_id: deal.primary_contact_id,
        primary_contact_display: contact_display_label(&state.db, deal.primary_contact_id).await,
        name: deal.name,
        amount: deal.amount.map(|a| a.to_string()).unwrap_or_default(),
        stage: deal.stage.as_str().to_string(),
        expected_close_date: deal
            .expected_close_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Form(form): Form<DealForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/deals").into_response();
    }
    let Some(existing) = find_deal_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/deals").into_response();
    };
    let company_id = form.company_id;
    let primary_contact_id = form.primary_contact_id;
    if company_id <= 0
        || primary_contact_id <= 0
        || !contact_belongs_to_company(&state.db, primary_contact_id, company_id).await
    {
        return Redirect::to("/crm/deals").into_response();
    }
    let now = Utc::now();
    let amount = if form.amount.trim().is_empty() {
        None
    } else {
        form.amount.parse::<rust_decimal::Decimal>().ok()
    };
    let mut am: deal::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.company_id = Set(company_id);
    am.primary_contact_id = Set(primary_contact_id);
    am.name = Set(form.name);
    am.amount = Set(amount);
    am.stage = Set(parse_deal_stage(&form.stage));
    am.expected_close_date = Set(parse_date(&form.expected_close_date));
    let _ = am.update(&state.db).await;
    Redirect::to(&DealDetailRouteTag::new(id).url()).into_response()
}

pub async fn delete_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/deals").into_response();
    }
    let _ = DealEntity::delete_by_id(id).exec(&state.db).await;
    Redirect::to("/crm/deals").into_response()
}
