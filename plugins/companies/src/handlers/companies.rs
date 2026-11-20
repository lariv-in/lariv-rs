use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::Condition;
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter};

use lariv_core::components::picker::respond_picker_select;
use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, opt_string};
use lariv_core::http::Cap;
use lariv_core::list_filters::{FilterField, apply_filters, clauses_from_query, text_column};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    path_and_query, respond_create_modal_done_fk, respond_edit_modal_done,
};
use lariv_plugin_users::auth;
use lariv_plugin_users::lookup::user_display_label;
use lariv_plugin_users::{middleware::RequireAuth, state::AuthContext};
use sea_orm::DatabaseConnection;

use crate::{
    entities::company::{self, Entity as CompanyEntity},
    forms::{CompanyForm, CompanyMemberForm},
    handlers::ModalNameQuery,
    hooks,
    keys::{
        COMPANY_CREATE_MODAL, COMPANY_DELETE_MODAL, COMPANY_EDIT_MODAL, COMPANY_SELECT_MODAL,
        COMPANY_SELECT_TABLE, COMPANY_TABLE,
    },
    logic::{self, CompanyWrite},
    routes::{CompanyDefaultRouteTag, CompanyDetailRouteTag},
    scope::{apply_company_sort, find_company_scoped, visible_companies},
    state::CompaniesState,
    templates::{
        CompanyCreateModalPage, CompanyDetailPage, CompanyEditModalPage, CompanyListPage,
        CompanyMemberRow, CompanyRow, CompanySelectPage, ConfirmDeletePage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct CompanyListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct CompanySelectQuery {
    #[serde(flatten)]
    pub filter: CompanyListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn companies_list_url() -> String {
    CompanyDefaultRouteTag.url()
}

fn company_fields() -> Vec<FilterField> {
    vec![text_column("name", "Name", company::Column::Name)]
}

async fn query_companies(
    db: &sea_orm::DatabaseConnection,
    q: &CompanyListQuery,
    _auth: &AuthContext,
    page_size: u32,
    condition: Condition,
) -> (Vec<company::Model>, u32, u64) {
    let mut query = visible_companies(scope_allowed::<super::super::routes::CompaniesView, _>(
        CompanyEntity::find(),
    ));
    query = query.filter(condition);
    query = apply_company_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

fn model_to_row(c: company::Model) -> CompanyRow {
    CompanyRow {
        id: c.id,
        name: c.name,
        website: c.website.unwrap_or_default(),
    }
}

async fn load_company_rows(
    db: &sea_orm::DatabaseConnection,
    q: &CompanyListQuery,
    auth: &AuthContext,
    page_size: u32,
    condition: Condition,
) -> ObjectList<CompanyRow> {
    let (models, page, total) = query_companies(db, q, auth, page_size, condition).await;
    let rows = models.into_iter().map(model_to_row).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<CompaniesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CompanyListQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""), None);
    let fields = company_fields();
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let companies =
        load_company_rows(&state.db, &q, &ctx, q.page_size.get(), applied.condition).await;
    let page = CompanyListPage {
        companies,
        fields,
        filters: FilterClauseView::from_clauses(&clauses, &applied.errors),
        filter_errors: applied.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(COMPANY_TABLE) {
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
    Cap(state): Cap<CompaniesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(company) = find_company_scoped(&state.db, id).await else {
        return Redirect::to(&companies_list_url()).into_response();
    };
    let is_owner = logic::user_owns_company(&state.db, id, ctx.user.id)
        .await
        .unwrap_or(false);
    let members = match logic::list_memberships_for_company(&state.db, id).await {
        Ok(rows) => {
            let mut out = Vec::new();
            for row in rows {
                let label = member_label(&state.db, row.user_id, ctx.user.id, &ctx.user.name).await;
                out.push(CompanyMemberRow {
                    membership_id: row.id,
                    label,
                    owned_by: row.owned_by,
                    is_self: row.user_id == ctx.user.id,
                });
            }
            out
        }
        Err(_) => Vec::new(),
    };
    let page = CompanyDetailPage {
        id: company.id,
        name: company.name,
        address_line_1: company.address_line_1.unwrap_or_default(),
        address_line_2: company.address_line_2.unwrap_or_default(),
        city: company.city.unwrap_or_default(),
        pincode: company.pincode.unwrap_or_default(),
        state: crate::gst_state::dropdown_value(&company.state.unwrap_or_default()),
        website: company.website.unwrap_or_default(),
        gstin: company.gstin.unwrap_or_default(),
        cin: company.cin.unwrap_or_default(),
        pan: company.pan.unwrap_or_default(),
        phone: company.phone.unwrap_or_default(),
        email: company.email.unwrap_or_default(),
        is_owner,
        members,
        member_error: String::new(),
        member_email: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn member_label(
    db: &DatabaseConnection,
    user_id: i64,
    viewer_id: i64,
    viewer_name: &str,
) -> String {
    if user_id == viewer_id {
        return format!("{viewer_name} (you)");
    }
    let name = user_display_label(db, user_id).await;
    if name.is_empty() {
        format!("User #{user_id}")
    } else {
        name
    }
}

fn company_write(form: &CompanyForm) -> Result<CompanyWrite, String> {
    Ok(CompanyWrite {
        name: form.name.clone(),
        address_line_1: opt_string(form.address_line_1.clone()),
        address_line_2: opt_string(form.address_line_2.clone()),
        city: opt_string(form.city.clone()),
        pincode: opt_string(form.pincode.clone()),
        state: crate::gst_state::parse_state(&form.state)?,
        website: opt_string(form.website.clone()),
        gstin: opt_string(form.gstin.clone()),
        cin: opt_string(form.cin.clone()),
        pan: opt_string(form.pan.clone()),
        phone: opt_string(form.phone.clone()),
        email: opt_string(form.email.clone()),
    })
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = CompanyCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        address_line_1: String::new(),
        address_line_2: String::new(),
        city: String::new(),
        pincode: String::new(),
        state: String::new(),
        website: String::new(),
        gstin: String::new(),
        cin: String::new(),
        pan: String::new(),
        phone: String::new(),
        email: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<CompaniesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<CompanyForm>,
) -> Response {
    let outcome = match company_write(&form) {
        Ok(company) => logic::create_company(&state.db, company, ctx.user.id).await,
        Err(error) => Err(error),
    };
    match outcome {
        Ok(saved) => {
            if let Err(err) =
                logic::set_active_company(&state.db, ctx.user.id, Some(saved.id)).await
            {
                tracing::error!(error = %err, company_id = saved.id, "set active company");
            }
            if let Err(err) = hooks::run_after_create(state.db.clone(), saved.id, ctx.user.id).await
            {
                tracing::error!(error = %err, company_id = saved.id, "after create company hook");
            }
            respond_create_modal_done_fk(
                &htmx,
                COMPANY_CREATE_MODAL,
                &q.refresh_table(),
                &CompanyDetailRouteTag::new(saved.id).url(),
                saved.id,
                &saved.name,
                &q.target_input(),
            )
        }
        Err(e) => {
            let page = CompanyCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                address_line_1: form.address_line_1,
                address_line_2: form.address_line_2,
                city: form.city,
                pincode: form.pincode,
                state: form.state,
                website: form.website,
                gstin: form.gstin,
                cin: form.cin,
                pan: form.pan,
                phone: form.phone,
                email: form.email,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<CompaniesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(company) = find_company_scoped(&state.db, id).await else {
        return Redirect::to(&companies_list_url()).into_response();
    };
    if !logic::user_owns_company(&state.db, id, ctx.user.id)
        .await
        .unwrap_or(false)
    {
        return Redirect::to(&CompanyDetailRouteTag::new(id).url()).into_response();
    }
    let page = CompanyEditModalPage {
        id: company.id,
        form_name: q.form_name(),
        name: company.name,
        address_line_1: company.address_line_1.unwrap_or_default(),
        address_line_2: company.address_line_2.unwrap_or_default(),
        city: company.city.unwrap_or_default(),
        pincode: company.pincode.unwrap_or_default(),
        state: crate::gst_state::dropdown_value(&company.state.unwrap_or_default()),
        website: company.website.unwrap_or_default(),
        gstin: company.gstin.unwrap_or_default(),
        cin: company.cin.unwrap_or_default(),
        pan: company.pan.unwrap_or_default(),
        phone: company.phone.unwrap_or_default(),
        email: company.email.unwrap_or_default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<CompaniesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<CompanyForm>,
) -> Response {
    if find_company_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&companies_list_url()).into_response();
    }
    if !logic::user_owns_company(&state.db, id, ctx.user.id)
        .await
        .unwrap_or(false)
    {
        return Redirect::to(&CompanyDetailRouteTag::new(id).url()).into_response();
    }
    let outcome = match company_write(&form) {
        Ok(company) => logic::update_company(&state.db, id, company).await,
        Err(error) => Err(error),
    };
    match outcome {
        Ok(_) => respond_edit_modal_done(
            &htmx,
            COMPANY_EDIT_MODAL,
            &CompanyDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = CompanyEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                address_line_1: form.address_line_1,
                address_line_2: form.address_line_2,
                city: form.city,
                pincode: form.pincode,
                state: form.state,
                website: form.website,
                gstin: form.gstin,
                cin: form.cin,
                pan: form.pan,
                phone: form.phone,
                email: form.email,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(state): Cap<CompaniesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> Response {
    if !logic::user_owns_company(&state.db, id, ctx.user.id)
        .await
        .unwrap_or(false)
    {
        return Redirect::to(&CompanyDetailRouteTag::new(id).url()).into_response();
    }
    let page = ConfirmDeletePage {
        modal_uid: COMPANY_DELETE_MODAL.id().to_string(),
        message: "Are you sure you want to delete this company?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_companies.CompanyDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn delete_post(
    Cap(state): Cap<CompaniesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !logic::user_owns_company(&state.db, id, ctx.user.id)
        .await
        .unwrap_or(false)
    {
        return Redirect::to(&CompanyDetailRouteTag::new(id).url()).into_response();
    }
    match logic::delete_company(&state.db, id).await {
        Ok(()) => htmx.redirect(&companies_list_url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete company");
            let page = ConfirmDeletePage {
                modal_uid: COMPANY_DELETE_MODAL.id().to_string(),
                message: "Are you sure you want to delete this company?".into(),
                form_name: "p_companies.CompanyDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn select(
    Cap(state): Cap<CompaniesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CompanySelectQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""), Some("name"));
    let fields = company_fields();
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let companies = load_company_rows(
        &state.db,
        &q.filter,
        &ctx,
        q.filter.page_size.get(),
        applied.condition,
    )
    .await;
    let page = CompanySelectPage {
        companies,
        fields,
        filters: FilterClauseView::from_clauses(&clauses, &applied.errors),
        filter_errors: applied.errors,
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "CompanyID".into()),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select(&htmx, COMPANY_SELECT_TABLE, COMPANY_SELECT_MODAL, &page)
}

pub async fn member_add_post(
    Cap(state): Cap<CompaniesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<CompanyMemberForm>,
) -> Response {
    let Some(company) = find_company_scoped(&state.db, id).await else {
        return Redirect::to(&companies_list_url()).into_response();
    };
    let is_owner = logic::user_owns_company(&state.db, id, ctx.user.id)
        .await
        .unwrap_or(false);
    if !is_owner {
        return Redirect::to(&CompanyDetailRouteTag::new(id).url()).into_response();
    }
    let email = form.email.trim();
    let outcome = async {
        let user = auth::find_by_email(&state.db, email)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("no user with email {email}"))?;
        if logic::membership_for_user(&state.db, id, user.id)
            .await?
            .is_some()
        {
            return Err("that user is already a member".to_string());
        }
        logic::add_membership(&state.db, id, user.id, false).await?;
        Ok::<(), String>(())
    }
    .await;
    match outcome {
        Ok(()) => htmx.redirect(&CompanyDetailRouteTag::new(id).url()),
        Err(error) => {
            let mut members = Vec::new();
            for row in logic::list_memberships_for_company(&state.db, id)
                .await
                .unwrap_or_default()
            {
                let label = member_label(&state.db, row.user_id, ctx.user.id, &ctx.user.name).await;
                members.push(CompanyMemberRow {
                    membership_id: row.id,
                    label,
                    owned_by: row.owned_by,
                    is_self: row.user_id == ctx.user.id,
                });
            }
            let page = CompanyDetailPage {
                id: company.id,
                name: company.name,
                address_line_1: company.address_line_1.unwrap_or_default(),
                address_line_2: company.address_line_2.unwrap_or_default(),
                city: company.city.unwrap_or_default(),
                pincode: company.pincode.unwrap_or_default(),
                state: crate::gst_state::dropdown_value(&company.state.unwrap_or_default()),
                website: company.website.unwrap_or_default(),
                gstin: company.gstin.unwrap_or_default(),
                cin: company.cin.unwrap_or_default(),
                pan: company.pan.unwrap_or_default(),
                phone: company.phone.unwrap_or_default(),
                email: company.email.unwrap_or_default(),
                is_owner,
                members,
                member_error: error,
                member_email: form.email,
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
    }
}

pub async fn member_remove_post(
    Cap(state): Cap<CompaniesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path((id, membership_id)): Path<(i64, i64)>,
) -> Response {
    if !logic::user_owns_company(&state.db, id, ctx.user.id)
        .await
        .unwrap_or(false)
    {
        return Redirect::to(&CompanyDetailRouteTag::new(id).url()).into_response();
    }
    match logic::remove_membership(&state.db, membership_id).await {
        Ok(()) => htmx.redirect(&CompanyDetailRouteTag::new(id).url()),
        Err(e) => {
            tracing::error!(error = %e, membership_id, "failed to remove membership");
            htmx.redirect(&CompanyDetailRouteTag::new(id).url())
        }
    }
}
