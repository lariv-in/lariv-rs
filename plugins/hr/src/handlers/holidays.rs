use lariv_plugin_users::role_authorization::scope_allowed;
use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::datetime::{format_date, parse_date};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_plugin_users::middleware::RequireAuth;
use lariv_core::web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
    };

use crate::{
    entities::holiday::{self, Entity as HolidayEntity},
    forms::HolidayForm,
    handlers::ModalNameQuery,
    keys::{HolidayCreateModalKey, HolidayEditModalKey, HolidayTableKey},
    logic::holiday::{HolidayInput, create_holiday, delete_holiday, update_holiday},
    routes::{HolidayDetailRouteTag, HolidayEditPostRouteTag, HolidayListRouteTag},
    state::HrState,
    templates::holidays::{
        HolidayCreateModalPage, HolidayDeleteModalPage, HolidayDetailPage, HolidayEditModalPage,
        HolidayListPage, HolidayRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct HolidayListQuery {
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(default, rename = "Date", alias = "date")]
    pub date: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn hub_query_from_uri(uri: &Uri) -> HolidayListQuery {
    let Some(query) = uri.query() else {
        return HolidayListQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

fn sort_desc(sort: &str) -> bool {
    sort.split_whitespace()
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case("DESC"))
}

pub async fn list(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    let q = hub_query_from_uri(&uri);
    let page_num = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.get();
    let mut query = scope_allowed::<super::super::routes::HolidayView, _>(HolidayEntity::find());
    if let Some(title) = q.title.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(holiday::Column::Title.contains(title));
    }
    if let Some(date) = q.date.as_deref().filter(|s| !s.is_empty()) {
        if let Some(date) = parse_date(date) {
            query = query.filter(holiday::Column::Date.eq(date));
        }
    }
    let sort = q.sort.as_deref().unwrap_or("");
    let desc = sort_desc(sort);
    query = match sort.split_whitespace().next().unwrap_or("") {
        s if s.eq_ignore_ascii_case("Title") => {
            if desc {
                query.order_by_desc(holiday::Column::Title)
            } else {
                query.order_by_asc(holiday::Column::Title)
            }
        }
        s if s.eq_ignore_ascii_case("Date") => {
            if desc {
                query.order_by_desc(holiday::Column::Date)
            } else {
                query.order_by_asc(holiday::Column::Date)
            }
        }
        _ => query
            .order_by_asc(holiday::Column::Date)
            .order_by_asc(holiday::Column::Id),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|holiday| HolidayRow {
            id: holiday.id,
            title: holiday.title,
            description: holiday.description,
            date: format_date(holiday.date),
            detail_href: HolidayDetailRouteTag::new(holiday.id).url(),
        })
        .collect();
    let page = HolidayListPage {
        rows: ObjectList::from_page(rows, page_num, page_size, total),
        filter_title: q.title.unwrap_or_default(),
        filter_date: q.date.unwrap_or_default(),
        sort: q.sort.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
    };
    if htmx.targets::<HolidayTableKey>() {
        return page.render_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let page = HolidayCreateModalPage::new(q.form_name(), q.refresh_table());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<HolidayForm>,
) -> Response {
    let input = match holiday_input_from_form(&form) {
        Ok(input) => input,
        Err(e) => {
            let page =
                HolidayCreateModalPage::with_form(q.form_name(), q.refresh_table(), &form, e);
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match create_holiday(&state.db, input).await {
        Ok(holiday) => respond_create_modal_done::<HolidayCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &HolidayDetailRouteTag::new(holiday.id).url(),
        ),
        Err(e) => {
            let page =
                HolidayCreateModalPage::with_form(q.form_name(), q.refresh_table(), &form, e);
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(holiday) = find_holiday_scoped(&state.db, id).await else {
        return Redirect::to(&HolidayListRouteTag.url()).into_response();
    };
    let page = HolidayDetailPage {
        id: holiday.id,
        title: holiday.title,
        description: holiday.description,
        date: format_date(holiday.date),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(holiday) = find_holiday_scoped(&state.db, id).await else {
        return Redirect::to(&HolidayListRouteTag.url()).into_response();
    };
    let page = HolidayEditModalPage {
        id: holiday.id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(HolidayEditPostRouteTag::new(holiday.id), &q.form_name()),
        title: holiday.title,
        description: holiday.description,
        date: format_date(holiday.date),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<HolidayForm>,
) -> Response {
    let input = match holiday_input_from_form(&form) {
        Ok(input) => input,
        Err(e) => {
            return edit_error_response(&chrome, &ctx, id, &q, &form, e).await;
        }
    };
    match update_holiday(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<HolidayEditModalKey>(
            &htmx,
            &HolidayDetailRouteTag::new(id).url(),
        ),
        Err(e) => edit_error_response(&chrome, &ctx, id, &q, &form, e).await,
    }
}

pub async fn delete_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_holiday_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&HolidayListRouteTag.url()).into_response();
    }
    let page = HolidayDeleteModalPage {
        id,
        form_name: q.form_name(),
        message: "Are you sure you want to delete this holiday?".into(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn delete_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    match delete_holiday(&state.db, id).await {
        Ok(()) => htmx.redirect(&HolidayListRouteTag.url()),
        Err(e) => {
            let page = HolidayDeleteModalPage {
                id,
                form_name: q.form_name(),
                message: "Are you sure you want to delete this holiday?".into(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

fn holiday_input_from_form(form: &HolidayForm) -> Result<HolidayInput, String> {
    let date_raw = form.date.trim();
    if date_raw.is_empty() {
        return Err("date is required".to_string());
    }
    let date = parse_date(date_raw).ok_or_else(|| "invalid date".to_string())?;
    Ok(HolidayInput {
        title: form.title.clone(),
        description: form.description.clone(),
        date,
    })
}

async fn edit_error_response(
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &HolidayForm,
    error: String,
) -> Response {
    let page = HolidayEditModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(HolidayEditPostRouteTag::new(id), &q.form_name()),
        title: form.title.clone(),
        description: form.description.clone(),
        date: form.date.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn find_holiday_scoped(db: &sea_orm::DatabaseConnection, id: i64) -> Option<holiday::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::super::routes::HolidayView, _>(HolidayEntity::find_by_id(id))
            .one(db)
            .await,
        "find holiday by id",
    )
}
