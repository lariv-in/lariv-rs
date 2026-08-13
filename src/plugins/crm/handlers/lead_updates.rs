use axum::{
    Form,
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::crm::{
    entities::lead_update::{self, Entity as LeadUpdateEntity},
    forms::LeadUpdateForm,
    handlers::ModalNameQuery,
    keys::{LeadUpdateCreateModalKey, LeadUpdateEditModalKey},
    routes::{LeadDetailRouteTag, LeadUpdateDetailRouteTag},
    scope::{
        apply_lead_update_sort, find_lead_scoped, find_lead_update_scoped, lead_display_name,
        scope_superuser, user_display_label, user_exists,
    },
    state::CrmState,
    templates::{
        LeadUpdateCreateModalPage, LeadUpdateDetailPage, LeadUpdateEditModalPage, LeadUpdateRow,
        LeadUpdatesTable,
    },
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct LeadUpdateListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

async fn query_updates(
    db: &sea_orm::DatabaseConnection,
    lead_id: i64,
    q: &LeadUpdateListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> (Vec<lead_update::Model>, u32, u64) {
    let mut query = LeadUpdateEntity::find().filter(lead_update::Column::LeadId.eq(lead_id));
    query = scope_superuser(query, auth);
    query = apply_lead_update_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

async fn model_to_row(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    u: lead_update::Model,
) -> LeadUpdateRow {
    LeadUpdateRow {
        id: u.id,
        datetime: auth.format_datetime(u.datetime).into_string(),
        created_by: user_display_label(db, u.created_by_id).await,
        description: u.description,
        detail_href: LeadUpdateDetailRouteTag::new(u.id).url(),
    }
}

pub(crate) async fn load_updates_table(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    lead_id: i64,
    q: &LeadUpdateListQuery,
    path_and_query: String,
    can_edit: bool,
) -> LeadUpdatesTable {
    let (models, page, total) = query_updates(db, lead_id, q, auth, PAGE_SIZE).await;
    let mut rows = Vec::with_capacity(models.len());
    for m in models {
        rows.push(model_to_row(db, auth, m).await);
    }
    LeadUpdatesTable {
        lead_id,
        updates: ObjectList::from_page(rows, page, PAGE_SIZE, total),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query,
        can_edit,
    }
}

fn lead_url(lead_id: i64) -> String {
    LeadDetailRouteTag::new(lead_id).url()
}

pub async fn detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(update) = find_lead_update_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let Some(lead) = find_lead_scoped(&state.db, update.lead_id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let page = LeadUpdateDetailPage {
        id: update.id,
        lead_id: update.lead_id,
        display_name: lead_display_name(&state.db, &lead).await,
        created_by: user_display_label(&state.db, update.created_by_id).await,
        datetime: ctx.format_datetime(update.datetime).into_string(),
        description: update.description,
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(lead_id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&lead_url(lead_id)).into_response();
    }
    if find_lead_scoped(&state.db, lead_id, &ctx).await.is_none() {
        return Redirect::to("/crm/leads").into_response();
    }
    let page = LeadUpdateCreateModalPage {
        lead_id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        created_by_id: ctx.user.id,
        created_by_display: ctx.user.name.clone(),
        datetime: ctx.datetime_local_input(Utc::now()).into_string(),
        description: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn created_by_id(form: &LeadUpdateForm, fallback: i64) -> i64 {
    if form.created_by_id <= 0 {
        fallback
    } else {
        form.created_by_id
    }
}

pub async fn create_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Path(lead_id): Path<i64>,
    Form(form): Form<LeadUpdateForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&lead_url(lead_id)).into_response();
    }
    if find_lead_scoped(&state.db, lead_id, &ctx).await.is_none() {
        return Redirect::to("/crm/leads").into_response();
    }
    let created_by_id = created_by_id(&form, ctx.user.id);
    let err = if !user_exists(&state.db, created_by_id).await {
        Some("created by is required")
    } else if form.description.trim().is_empty() {
        Some("description is required")
    } else {
        None
    };
    if let Some(error) = err {
        let page = LeadUpdateCreateModalPage {
            lead_id,
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            created_by_id,
            created_by_display: user_display_label(&state.db, created_by_id).await,
            datetime: form.datetime,
            description: form.description,
            error: error.to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        let page = LeadUpdateCreateModalPage {
            lead_id,
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            created_by_id,
            created_by_display: user_display_label(&state.db, created_by_id).await,
            datetime: form.datetime,
            description: form.description,
            error: "invalid date & time".to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    let now = Utc::now();
    let model = lead_update::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        lead_id: Set(lead_id),
        created_by_id: Set(created_by_id),
        datetime: Set(datetime),
        description: Set(form.description.trim().to_string()),
    };
    match model.insert(&state.db).await {
        Ok(_) => respond_create_modal_done::<LeadUpdateCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &lead_url(lead_id),
        ),
        Err(e) => {
            let page = LeadUpdateCreateModalPage {
                lead_id,
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                created_by_id,
                created_by_display: user_display_label(&state.db, created_by_id).await,
                datetime: form.datetime,
                description: form.description,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
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
    let Some(update) = find_lead_update_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let page = LeadUpdateEditModalPage {
        id: update.id,
        form_name: q.form_name(),
        created_by_id: update.created_by_id,
        created_by_display: user_display_label(&state.db, update.created_by_id).await,
        datetime: ctx.datetime_local_input(update.datetime).into_string(),
        description: update.description,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn edit_modal_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &LeadUpdateForm,
    created_by_id: i64,
    error: &str,
) -> Response {
    let page = LeadUpdateEditModalPage {
        id,
        form_name: q.form_name(),
        created_by_id,
        created_by_display: user_display_label(db, created_by_id).await,
        datetime: form.datetime.clone(),
        description: form.description.clone(),
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
    Form(form): Form<LeadUpdateForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    let Some(existing) = find_lead_update_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let created_by_id = created_by_id(&form, existing.created_by_id);
    if !user_exists(&state.db, created_by_id).await {
        return edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            created_by_id,
            "created by is required",
        )
        .await;
    }
    if form.description.trim().is_empty() {
        return edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            created_by_id,
            "description is required",
        )
        .await;
    }
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        return edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            created_by_id,
            "invalid date & time",
        )
        .await;
    };
    let now = Utc::now();
    let mut am: lead_update::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.created_by_id = Set(created_by_id);
    am.datetime = Set(datetime);
    am.description = Set(form.description.trim().to_string());
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<LeadUpdateEditModalKey>(
            &htmx,
            &LeadUpdateDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            edit_modal_error(
                &state.db,
                &chrome,
                &ctx,
                id,
                &q,
                &form,
                created_by_id,
                &e.to_string(),
            )
            .await
        }
    }
}

pub async fn delete_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/leads").into_response();
    }
    let Some(update) = find_lead_update_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/leads").into_response();
    };
    let lead_id = update.lead_id;
    let _ = LeadUpdateEntity::delete_by_id(id).exec(&state.db).await;
    Redirect::to(&lead_url(lead_id)).into_response()
}
