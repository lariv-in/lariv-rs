use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::{
    components::{
        DEFAULT_PAGE_SIZE, ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey,
    },
    html_form::{HtmlFormBody, UrlencodedFields, form_vec_i64},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    template::RenderAppPane,
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done,
        respond_edit_modal_done,
    },
};

use crate::plugins::crm::{
    entities::{
        converted_lead::{self, Entity as ConvertedLeadEntity},
        failed_lead::{self, Entity as FailedLeadEntity},
        lead::Entity as LeadEntity,
    },
    forms::{ConvertLeadBody, FailLeadForm, LeadEditBody, LeadForm},
    handlers::{
        ModalNameQuery,
        lead_tags::{load_tag_items_for_lead, load_tags_for_lead, tag_items_from_ids},
        lead_updates::load_updates_panel,
    },
    keys::{
        LeadConvertModalKey, LeadCreateModalKey, LeadDeleteModalKey, LeadEditModalKey,
        LeadFailModalKey, LeadHubTableKey, LeadUpdatesKey,
    },
    lead_source::LeadSource,
    logic::{
        lead::{LeadInput, create_lead, delete_lead, update_lead},
        lead_conversion::{convert_lead, unconvert_lead},
        lead_fail::{fail_lead, reactivate_lead, update_failed_reason},
    },
    routes::{ConvertedLeadDetailRouteTag, FailedLeadDetailRouteTag, LeadDetailRouteTag},
    scope::{
        apply_converted_lead_sort, apply_failed_lead_sort, apply_lead_filters, apply_lead_sort,
        apply_lead_tag_id_filter, company_display_label, contact_display_label, find_active_lead,
        find_contact_scoped, find_converted_lead_scoped, find_failed_lead_scoped, find_lead_scoped,
        lead_contact_view, sql_lead_active,
    },
    state::CrmState,
    templates::{
        ConfirmDeletePage, ConvertLeadModalPage, FailLeadModalPage, LeadConvertDetailPage,
        LeadCreateModalPage, LeadDetailPage, LeadEditModalPage, LeadFailDetailPage, LeadHubPage,
        LeadRow, LeadTagChip,
    },
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct HubQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default, rename = "CompanyID", alias = "company_id")]
    pub company_id: Option<String>,
    #[serde(default, rename = "Contact", alias = "contact")]
    pub contact: Option<String>,
    #[serde(
        default,
        rename = "Tags",
        alias = "tags",
        deserialize_with = "form_vec_i64"
    )]
    pub tags: Vec<i64>,
    #[serde(default)]
    pub sort: Option<String>,
}

fn hub_query_from_uri(uri: &Uri) -> HubQuery {
    let Some(query) = uri.query() else {
        return HubQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn opt_string(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

fn parse_i64(raw: Option<&str>) -> Option<i64> {
    raw.and_then(|s| s.trim().parse().ok()).filter(|id| *id > 0)
}

fn parse_lead_source(raw: &str) -> Option<LeadSource> {
    LeadSource::parse(raw)
}

fn source_label(source: Option<LeadSource>) -> String {
    source.map(|s| s.label().to_string()).unwrap_or_default()
}

fn source_value(source: Option<LeadSource>) -> String {
    source.map(|s| s.as_str().to_string()).unwrap_or_default()
}

fn convert_modal_page_from_body(
    lead_id: i64,
    q: &ModalNameQuery,
    error: String,
) -> ConvertLeadModalPage {
    ConvertLeadModalPage {
        lead_id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        error,
    }
}

fn lead_input_from_form(form: &LeadForm) -> LeadInput {
    LeadInput {
        contact_id: form.contact_id,
        source: parse_lead_source(&form.source),
        notes: opt_string(form.notes.clone()),
        tag_ids: form.tags.clone(),
    }
}

async fn lead_tag_chips(db: &sea_orm::DatabaseConnection, lead_id: i64) -> Vec<LeadTagChip> {
    load_tags_for_lead(db, lead_id)
        .await
        .into_iter()
        .map(|t| LeadTagChip {
            id: t.id,
            name: t.name,
            color: t.color,
        })
        .collect()
}

fn lead_create_modal_page(
    q: &ModalNameQuery,
    contact_id: i64,
    contact_display: String,
    source: String,
    notes: String,
    tags: Vec<ManyToManyItem>,
    error: String,
) -> LeadCreateModalPage {
    LeadCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        contact_id,
        contact_display,
        source,
        notes,
        tags,
        error,
    }
}

async fn lead_return_url(db: &sea_orm::DatabaseConnection, lead_id: i64) -> String {
    if let Some(c) = crate::web::opt_or_log(
        ConvertedLeadEntity::find()
            .filter(converted_lead::Column::LeadId.eq(lead_id))
            .one(db)
            .await,
        "db find one",
    ) {
        return ConvertedLeadDetailRouteTag::new(c.id).url();
    }
    if let Some(f) = crate::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(lead_id))
            .one(db)
            .await,
        "db find one",
    ) {
        return FailedLeadDetailRouteTag::new(f.id).url();
    }
    LeadDetailRouteTag::new(lead_id).url()
}

pub(crate) async fn query_active_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = LeadEntity::find().filter(sql_lead_active());
    query = apply_lead_filters(
        query,
        parse_i64(q.company_id.as_deref()),
        q.contact.as_deref(),
        &q.tags,
        q.sort.as_deref(),
    );
    query = apply_lead_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for l in models {
        let view = lead_contact_view(db, l.contact_id).await;
        rows.push(LeadRow {
            id: l.id,
            name: if view.display_name.is_empty() {
                format!("Lead #{}", l.id)
            } else {
                view.display_name
            },
            company: view.company,
            email: view.email,
            source: source_label(l.source),
            status: "Active".to_string(),
            detail_href: LeadDetailRouteTag::new(l.id).url(),
        });
    }
    (rows, page_num, total)
}

pub(crate) async fn query_converted_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = ConvertedLeadEntity::find();
    query = apply_lead_tag_id_filter(query, converted_lead::Column::LeadId, &q.tags);
    let query = apply_converted_lead_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let converted = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let company_filter = parse_i64(q.company_id.as_deref());
    let contact_filter = q.contact.as_deref().filter(|s| !s.is_empty());
    let mut rows = Vec::new();
    for c in converted {
        let lead = crate::web::opt_or_log(
            LeadEntity::find_by_id(c.lead_id).one(db).await,
            "find by id",
        );
        let (name, company, email, source, company_id) = match lead {
            Some(l) => {
                let view = lead_contact_view(db, l.contact_id).await;
                (
                    if view.display_name.is_empty() {
                        format!("Lead #{}", l.id)
                    } else {
                        view.display_name
                    },
                    view.company,
                    view.email,
                    source_label(l.source),
                    view.company_id,
                )
            }
            None => (
                format!("Lead #{}", c.lead_id),
                String::new(),
                String::new(),
                String::new(),
                0,
            ),
        };
        if let Some(cid) = company_filter {
            if company_id != cid {
                continue;
            }
        }
        if let Some(cf) = contact_filter {
            let hay = format!("{name} {email}").to_lowercase();
            if !hay.contains(&cf.to_lowercase()) {
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

pub(crate) async fn query_failed_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = FailedLeadEntity::find();
    query = apply_lead_tag_id_filter(query, failed_lead::Column::LeadId, &q.tags);
    query = apply_failed_lead_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let failed = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::new();
    for f in failed {
        let lead = crate::web::opt_or_log(
            LeadEntity::find_by_id(f.lead_id).one(db).await,
            "find by id",
        );
        let (name, company, email, source) = match lead {
            Some(l) => {
                let view = lead_contact_view(db, l.contact_id).await;
                (
                    if view.display_name.is_empty() {
                        format!("Lead #{}", l.id)
                    } else {
                        view.display_name
                    },
                    view.company,
                    view.email,
                    source_label(l.source),
                )
            }
            None => (
                format!("Lead #{}", f.lead_id),
                String::new(),
                String::new(),
                String::new(),
            ),
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
) -> maud::Markup {
    let q = hub_query_from_uri(&uri);
    let tab = q.tab.as_deref().unwrap_or("active").to_string();
    let (rows, page, total) = match tab.as_str() {
        "converted" => query_converted_leads(&state.db, &q, PAGE_SIZE).await,
        "failed" => query_failed_leads(&state.db, &q, PAGE_SIZE).await,
        _ => query_active_leads(&state.db, &q, PAGE_SIZE).await,
    };
    let filter_company_id = parse_i64(q.company_id.as_deref()).unwrap_or(0);
    let leads = ObjectList::from_page(rows, page, PAGE_SIZE, total);
    let page = LeadHubPage {
        leads,
        tab,
        filter_company_id,
        filter_company_display: company_display_label(&state.db, filter_company_id).await,
        filter_contact: q.contact.clone().unwrap_or_default(),
        filter_tags: tag_items_from_ids(&state.db, &q.tags).await,
        sort: q.sort.clone().unwrap_or_default(),
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
    let page = lead_create_modal_page(
        &q,
        0,
        String::new(),
        String::new(),
        String::new(),
        Vec::new(),
        String::new(),
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<LeadForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    if form.contact_id <= 0
        || find_contact_scoped(&state.db, form.contact_id, &ctx)
            .await
            .is_none()
    {
        let page = lead_create_modal_page(
            &q,
            form.contact_id,
            contact_display_label(&state.db, form.contact_id).await,
            form.source,
            form.notes,
            tag_items_from_ids(&state.db, &form.tags).await,
            "contact is required".to_string(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    match create_lead(&state.db, lead_input_from_form(&form)).await {
        Ok(saved) => respond_create_modal_done::<LeadCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &LeadDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = lead_create_modal_page(
                &q,
                form.contact_id,
                contact_display_label(&state.db, form.contact_id).await,
                form.source,
                form.notes,
                tag_items_from_ids(&state.db, &form.tags).await,
                e,
            );
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
        if let Some(c) = crate::web::opt_or_log(
            ConvertedLeadEntity::find()
                .filter(converted_lead::Column::LeadId.eq(id))
                .one(&state.db)
                .await,
            "db find one",
        ) {
            return Redirect::to(&ConvertedLeadDetailRouteTag::new(c.id).url()).into_response();
        }
        if let Some(f) = crate::web::opt_or_log(
            FailedLeadEntity::find()
                .filter(failed_lead::Column::LeadId.eq(id))
                .one(&state.db)
                .await,
            "db find one",
        ) {
            return Redirect::to(&FailedLeadDetailRouteTag::new(f.id).url()).into_response();
        }
        return Redirect::to("/crm/leads").into_response();
    }
    let view = lead_contact_view(&state.db, lead.contact_id).await;
    let can_edit = ctx.user.is_superuser;
    let page = LeadDetailPage {
        id: lead.id,
        display_name: if view.display_name.is_empty() {
            format!("Lead #{}", lead.id)
        } else {
            view.display_name
        },
        contact_id: view.contact_id,
        contact_display: contact_display_label(&state.db, view.contact_id).await,
        company_id: view.company_id,
        company: view.company,
        email: view.email,
        source: source_label(lead.source),
        notes: lead.notes.unwrap_or_default(),
        tags: lead_tag_chips(&state.db, lead.id).await,
        can_edit,
        updates: load_updates_panel(&state.db, &ctx, lead.id, can_edit).await,
    };
    if htmx.targets::<LeadUpdatesKey>() {
        return page.updates.render_list().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    let Some(lead) = find_lead_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let failed = crate::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(id))
            .one(&state.db)
            .await,
        "db find one",
    );
    let page = LeadEditModalPage {
        id: lead.id,
        form_name: q.form_name(),
        contact_id: lead.contact_id,
        contact_display: contact_display_label(&state.db, lead.contact_id).await,
        source: source_value(lead.source),
        notes: lead.notes.unwrap_or_default(),
        tags: load_tag_items_for_lead(&state.db, lead.id).await,
        reason: failed
            .as_ref()
            .and_then(|f| f.reason.clone())
            .unwrap_or_default(),
        show_reason: failed.is_some(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn lead_edit_modal_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &crate::plugins::users::state::AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &LeadEditBody,
    error: &str,
) -> Response {
    let show_reason = crate::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(id))
            .one(db)
            .await,
        "db find one",
    )
    .is_some();
    let page = LeadEditModalPage {
        id,
        form_name: q.form_name(),
        contact_id: form.lead.contact_id,
        contact_display: contact_display_label(db, form.lead.contact_id).await,
        source: form.lead.source.clone(),
        notes: form.lead.notes.clone(),
        tags: tag_items_from_ids(db, &form.lead.tags).await,
        reason: form.reason.clone(),
        show_reason,
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<LeadEditBody>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    if find_lead_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to("/crm/leads").into_response();
    }
    if form.lead.contact_id <= 0
        || find_contact_scoped(&state.db, form.lead.contact_id, &ctx)
            .await
            .is_none()
    {
        return lead_edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            "contact is required",
        )
        .await;
    }
    if let Err(e) = update_lead(&state.db, id, lead_input_from_form(&form.lead)).await {
        return lead_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e).await;
    }
    if crate::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(id))
            .one(&state.db)
            .await,
        "db find one",
    )
    .is_some()
    {
        if let Err(e) =
            update_failed_reason(&state.db, id, &ctx, opt_string(form.reason.clone())).await
        {
            return lead_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e).await;
        }
    }
    respond_edit_modal_done::<LeadEditModalKey>(&htmx, &lead_return_url(&state.db, id).await)
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: LeadDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this lead?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_crm.LeadDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    match delete_lead(&state.db, id).await {
        Ok(()) => htmx.redirect("/crm/leads"),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete lead");
            let page = ConfirmDeletePage {
                modal_uid: LeadDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this lead?".into(),
                form_name: "p_crm.LeadDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
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
    HtmlFormBody(_form): HtmlFormBody<ConvertLeadBody>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    match convert_lead(&state.db, id, &ctx).await {
        Ok(result) => respond_create_modal_done::<LeadConvertModalKey>(
            &htmx,
            &q.refresh_table(),
            &ConvertedLeadDetailRouteTag::new(result.converted_id).url(),
        ),
        Err(e) => {
            let page = convert_modal_page_from_body(id, &q, e);
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
    let Some(lead) = find_lead_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let already_failed = crate::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(lead.id))
            .one(&state.db)
            .await,
        "db find one",
    )
    .is_some();
    if already_failed {
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
    HtmlFormBody(form): HtmlFormBody<FailLeadForm>,
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
    let view = match &lead {
        Some(l) => lead_contact_view(&state.db, l.contact_id).await,
        None => Default::default(),
    };
    let display_name = if view.display_name.is_empty() {
        format!("Lead #{}", converted.lead_id)
    } else {
        view.display_name.clone()
    };
    let can_edit = ctx.user.is_superuser;
    let page = LeadConvertDetailPage {
        converted_id: converted.id,
        lead_id: converted.lead_id,
        display_name,
        converted_at: converted
            .converted_at
            .format("%d/%m/%Y %H:%M UTC")
            .to_string(),
        company_id: converted.company_id,
        contact_id: converted.contact_id,
        company: company_display_label(&state.db, converted.company_id).await,
        contact_display: contact_display_label(&state.db, converted.contact_id).await,
        email: view.email,
        source: lead
            .as_ref()
            .map(|l| source_label(l.source))
            .unwrap_or_default(),
        notes: lead
            .as_ref()
            .and_then(|l| l.notes.clone())
            .unwrap_or_default(),
        tags: lead_tag_chips(&state.db, converted.lead_id).await,
        can_edit,
        updates: load_updates_panel(&state.db, &ctx, converted.lead_id, can_edit).await,
    };
    if htmx.targets::<LeadUpdatesKey>() {
        return page.updates.render_list().into_response();
    }
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
    let view = match &lead {
        Some(l) => lead_contact_view(&state.db, l.contact_id).await,
        None => Default::default(),
    };
    let display_name = if view.display_name.is_empty() {
        format!("Lead #{}", failed.lead_id)
    } else {
        view.display_name.clone()
    };
    let can_edit = ctx.user.is_superuser;
    let page = LeadFailDetailPage {
        failed_id: failed.id,
        lead_id: failed.lead_id,
        display_name,
        failed_at: failed.failed_at.format("%d/%m/%Y %H:%M UTC").to_string(),
        reason: failed.reason.unwrap_or_default(),
        contact_id: view.contact_id,
        contact_display: contact_display_label(&state.db, view.contact_id).await,
        company_id: view.company_id,
        company: view.company,
        email: view.email,
        source: lead
            .as_ref()
            .map(|l| source_label(l.source))
            .unwrap_or_default(),
        notes: lead
            .as_ref()
            .and_then(|l| l.notes.clone())
            .unwrap_or_default(),
        tags: lead_tag_chips(&state.db, failed.lead_id).await,
        can_edit,
        updates: load_updates_panel(&state.db, &ctx, failed.lead_id, can_edit).await,
    };
    if htmx.targets::<LeadUpdatesKey>() {
        return page.updates.render_list().into_response();
    }
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

pub async fn converted_reactivate_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    match unconvert_lead(&state.db, id, &ctx).await {
        Ok(lead_id) => Redirect::to(&LeadDetailRouteTag::new(lead_id).url()).into_response(),
        Err(_) => Redirect::to("/crm/leads").into_response(),
    }
}
