use axum::{
    extract::{Form, Path, Query},
    http::{HeaderValue, Uri},
    response::{IntoResponse, Redirect, Response},
};
use maud::Markup;
use sea_orm::{
    ColumnTrait, DatabaseBackend, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Related, Select,
};

use lariv_core::list_filters::clauses_for;

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::{
    entities::user, middleware::RequireAuth, roles::Superuser, state::AuthContext,
};

use crate::{
    entities::attendance::{AttendanceColumns, Record, postgres, sqlite},
    forms::AttendanceForm,
    handlers::ModalNameQuery,
    keys::{AttendanceCreateModalKey, AttendanceEditModalKey, AttendanceTableKey},
    logic::{
        attendance::{
            AttendanceInput, PunchOutcome, create_attendance, delete_attendance, format_distance,
            format_position, parse_reported_position, punch_availability, punch_in, punch_out,
            punch_subject, update_attendance, user_display_label, user_display_labels,
        },
        dashboard::employee_for_user,
        preferences::load_attendance_zone,
    },
    routes::{AttendanceDetailRouteTag, AttendanceEditPostRouteTag, AttendanceListRouteTag},
    state::HrState,
    templates::attendances::{
        AttendanceCreateModalPage, AttendanceDeleteModalPage, AttendanceDetailPage,
        AttendanceEditModalPage, AttendanceListPage, AttendancePunchBar, AttendanceRow,
        outside_zone_modal,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct AttendanceListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    /// Query values arrive as strings (`page=2`). [`QueryPage`] parses that;
    /// `Option<u32>` rejects it and the whole query falls back to page 1.
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn hub_query_from_uri(uri: &Uri) -> AttendanceListQuery {
    let Some(query) = uri.query() else {
        return AttendanceListQuery::default();
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

fn parse_user_id(raw: Option<&str>) -> Option<i64> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    raw.parse::<i64>().ok().filter(|id| *id > 0)
}

pub async fn list(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    render_list(&state, &chrome, &ctx, &htmx, &uri, String::new()).await
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct PunchCoords {
    #[serde(default)]
    latitude: String,
    #[serde(default)]
    longitude: String,
}

pub async fn punch_in_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Form(coords): Form<PunchCoords>,
) -> Response {
    punch_and_render(&state, &chrome, &ctx, &htmx, true, coords).await
}

pub async fn punch_out_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Form(coords): Form<PunchCoords>,
) -> Response {
    punch_and_render(&state, &chrome, &ctx, &htmx, false, coords).await
}

async fn punch_and_render(
    state: &HrState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    htmx: &Htmx,
    punching_in: bool,
    coords: PunchCoords,
) -> Response {
    let uri = current_list_uri(htmx);
    let clauses = lariv_core::list_filters::parse_list_filters(uri.query().unwrap_or(""));
    let filter_user = clauses_for(&clauses, "user_id")
        .first()
        .map(|clause| clause.value().to_string());
    let position = match parse_reported_position(&coords.latitude, &coords.longitude) {
        Ok(position) => position,
        Err(error) => return render_list(state, chrome, ctx, htmx, &uri, error).await,
    };
    let outcome = match punch_user_id(
        Superuser::matches(&ctx.role),
        ctx.user.id,
        filter_user.as_deref(),
    ) {
        Ok(user_id) => {
            if punching_in {
                punch_in(&state.db, user_id, position).await
            } else {
                punch_out(&state.db, user_id, position).await
            }
        }
        Err(error) => return render_list(state, chrome, ctx, htmx, &uri, error).await,
    };
    match outcome {
        Ok(PunchOutcome::Done(_)) => {
            render_list(state, chrome, ctx, htmx, &uri, String::new()).await
        }
        Ok(PunchOutcome::MissingLocation) => {
            render_list(
                state,
                chrome,
                ctx,
                htmx,
                &uri,
                "Location is required to punch in or out.".to_string(),
            )
            .await
        }
        Ok(PunchOutcome::Outside {
            point,
            zone,
            distance_meters,
        }) => retarget_modal(outside_zone_modal(
            point,
            zone.center,
            zone.range_meters,
            distance_meters,
            &format!(
                "You are {} from the attendance location. Punch in and punch out are accepted within {}.",
                format_distance(distance_meters),
                format_distance(zone.range_meters),
            ),
        )),
        Err(error) => render_list(state, chrome, ctx, htmx, &uri, error).await,
    }
}

fn retarget_modal(markup: Markup) -> Response {
    let mut response = markup.into_response();
    let headers = response.headers_mut();
    headers.insert("HX-Retarget", HeaderValue::from_static("body"));
    headers.insert("HX-Reswap", HeaderValue::from_static("beforeend"));
    headers.insert(
        "HX-Reselect",
        HeaderValue::from_static("#hr-attendance-outside-modal"),
    );
    response
}

async fn render_list(
    state: &HrState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    htmx: &Htmx,
    uri: &Uri,
    error: String,
) -> Response {
    let q = hub_query_from_uri(uri);
    let page_num = q.page.get();
    let page_size = q.page_size.get();
    let is_superuser = Superuser::matches(&ctx.role);
    let own_employee = !is_superuser && employee_for_user(&state.db, ctx.user.id).await.is_some();
    let mut prepared = crate::filters::prepare(
        uri.query().unwrap_or(""),
        crate::filters::attendance_fields(is_superuser, &ctx.timezone),
        state.db.get_database_backend(),
    );
    crate::filters::fill_user_fk(&state.db, &mut prepared.filters, "user_id").await;
    match state.db.get_database_backend() {
        DatabaseBackend::Postgres => {
            render_list_entity::<postgres::Entity>(
                state,
                chrome,
                ctx,
                htmx,
                uri,
                error,
                is_superuser,
                own_employee,
                page_num,
                page_size,
                q,
                prepared,
            )
            .await
        }
        _ => {
            render_list_entity::<sqlite::Entity>(
                state,
                chrome,
                ctx,
                htmx,
                uri,
                error,
                is_superuser,
                own_employee,
                page_num,
                page_size,
                q,
                prepared,
            )
            .await
        }
    }
}

async fn render_list_entity<E>(
    state: &HrState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    htmx: &Htmx,
    uri: &Uri,
    error: String,
    is_superuser: bool,
    own_employee: bool,
    page_num: u32,
    page_size: u32,
    q: AttendanceListQuery,
    prepared: crate::filters::PreparedFilters,
) -> Response
where
    E: EntityTrait + Related<user::Entity>,
    E::Column: AttendanceColumns,
    E::Model: Into<Record> + Send + Sync,
{
    let mut query = scope_attendance::<E>(E::find(), is_superuser, own_employee, ctx.user.id)
        .filter(prepared.condition);
    let sort = q.sort.as_deref().unwrap_or("");
    let desc = sort_desc(sort);
    let sort_key = sort.split_whitespace().next().unwrap_or("");
    if sort_key.eq_ignore_ascii_case("User") {
        query = query.join(JoinType::LeftJoin, E::to());
    }
    query = match sort_key {
        s if s.eq_ignore_ascii_case("User") => {
            if desc {
                query.order_by_desc(user::Column::Name)
            } else {
                query.order_by_asc(user::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("StartedAt") => {
            if desc {
                query.order_by_desc(E::Column::col_started_at())
            } else {
                query.order_by_asc(E::Column::col_started_at())
            }
        }
        s if s.eq_ignore_ascii_case("EndedAt") => {
            if desc {
                query.order_by_desc(E::Column::col_ended_at())
            } else {
                query.order_by_asc(E::Column::col_ended_at())
            }
        }
        _ => query
            .order_by_desc(E::Column::col_started_at())
            .order_by_desc(E::Column::col_id()),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let records: Vec<Record> = models.into_iter().map(Into::into).collect();
    let names = user_display_labels(
        &state.db,
        &records.iter().map(|row| row.user_id).collect::<Vec<_>>(),
    )
    .await;
    let rows = records
        .into_iter()
        .map(|row| {
            let user = names
                .get(&row.user_id)
                .cloned()
                .unwrap_or_else(|| format!("User {}", row.user_id));
            AttendanceRow {
                id: row.id,
                user,
                started_at: ctx.format_datetime(row.started_at).into_string(),
                ended_at: format_optional_datetime(ctx, row.ended_at),
                position: format_position(row.started, row.ended),
                detail_href: AttendanceDetailRouteTag::new(row.id).url(),
            }
        })
        .collect();
    let filter_user = clauses_for(&prepared.clauses, "user_id")
        .first()
        .map(|clause| clause.value().to_string());
    let filter_user_id = if is_superuser {
        parse_user_id(filter_user.as_deref()).unwrap_or(0)
    } else if own_employee {
        ctx.user.id
    } else {
        0
    };
    let rows = ObjectList::from_page(rows, page_num, page_size, total);
    let sort = q.sort.clone().unwrap_or_default();
    let path_and_query = path_and_query(uri);
    let show_punch = own_employee || (is_superuser && filter_user_id > 0);
    let punch = if show_punch {
        Some(punch_bar(&state, filter_user_id).await)
    } else {
        None
    };
    let tz = ctx.timezone.clone();
    if htmx.targets::<AttendanceTableKey>() {
        let page = AttendanceListPage {
            rows,
            fields: prepared.fields,
            filters: prepared.filters,
            filter_errors: prepared.errors,
            sort,
            path_and_query,
            page_size,
            error,
            punch,
        };
        return page.render_table().into_response();
    }
    let filters = prepared.filters;
    let filter_errors = prepared.errors;
    drop(prepared.fields);
    crate::nav::hr_page(&state.db, ctx.user.id, || {
        let page = AttendanceListPage {
            rows,
            fields: crate::filters::attendance_fields(is_superuser, &tz),
            filters,
            filter_errors,
            sort,
            path_and_query,
            page_size,
            error,
            punch,
        };
        html_built_page_or_app_layout(&page, htmx, chrome, &SlotCtx::from_auth(ctx)).into_response()
    })
    .await
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let page = AttendanceCreateModalPage::new(q.form_name(), q.refresh_table());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<AttendanceForm>,
) -> Response {
    let user_display = user_display_label(&state.db, form.user_id).await;
    let input = match attendance_input_from_form(&ctx, &form) {
        Ok(input) => input,
        Err(e) => {
            let page = AttendanceCreateModalPage::with_form(
                q.form_name(),
                q.refresh_table(),
                &form,
                user_display,
                e,
            );
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match create_attendance(&state.db, input).await {
        Ok(row) => respond_create_modal_done::<AttendanceCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &AttendanceDetailRouteTag::new(row.id).url(),
        ),
        Err(e) => {
            let page = AttendanceCreateModalPage::with_form(
                q.form_name(),
                q.refresh_table(),
                &form,
                user_display,
                e,
            );
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
    let Some(row) = find_attendance_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&AttendanceListRouteTag.url()).into_response();
    };
    let user = user_display_label(&state.db, row.user_id).await;
    let page = AttendanceDetailPage {
        id: row.id,
        user,
        started_at: ctx.format_datetime(row.started_at).into_string(),
        ended_at: format_optional_datetime(&ctx, row.ended_at),
        position: format_position(row.started, row.ended),
        started_latitude: coord_text(row.started.map(|point| point.latitude)),
        started_longitude: coord_text(row.started.map(|point| point.longitude)),
        ended_latitude: coord_text(row.ended.map(|point| point.latitude)),
        ended_longitude: coord_text(row.ended.map(|point| point.longitude)),
    };
    crate::nav::hr_page(&state.db, ctx.user.id, || {
        html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response()
    })
    .await
}

pub async fn edit_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(row) = find_attendance_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&AttendanceListRouteTag.url()).into_response();
    };
    let page = AttendanceEditModalPage {
        id: row.id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(AttendanceEditPostRouteTag::new(row.id), &q.form_name()),
        user_id: row.user_id,
        user_display: user_display_label(&state.db, row.user_id).await,
        started_at: ctx.datetime_local_input(row.started_at).into_string(),
        ended_at: row
            .ended_at
            .map(|dt| ctx.datetime_local_input(dt).into_string())
            .unwrap_or_default(),
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
    HtmlFormBody(form): HtmlFormBody<AttendanceForm>,
) -> Response {
    let user_display = user_display_label(&state.db, form.user_id).await;
    let input = match attendance_input_from_form(&ctx, &form) {
        Ok(input) => input,
        Err(e) => {
            return edit_error_response(&chrome, &ctx, id, &q, &form, &user_display, e).await;
        }
    };
    match update_attendance(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<AttendanceEditModalKey>(
            &htmx,
            &AttendanceDetailRouteTag::new(id).url(),
        ),
        Err(e) => edit_error_response(&chrome, &ctx, id, &q, &form, &user_display, e).await,
    }
}

pub async fn delete_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_attendance_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&AttendanceListRouteTag.url()).into_response();
    }
    let page = AttendanceDeleteModalPage {
        id,
        form_name: q.form_name(),
        message: "Are you sure you want to delete this attendance?".into(),
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
    match delete_attendance(&state.db, id).await {
        Ok(()) => htmx.redirect(&AttendanceListRouteTag.url()),
        Err(e) => {
            let page = AttendanceDeleteModalPage {
                id,
                form_name: q.form_name(),
                message: "Are you sure you want to delete this attendance?".into(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

fn attendance_input_from_form(
    ctx: &AuthContext,
    form: &AttendanceForm,
) -> Result<AttendanceInput, String> {
    if form.user_id <= 0 {
        return Err("user is required".to_string());
    }
    let started_raw = form.started_at.trim();
    if started_raw.is_empty() {
        return Err("start is required".to_string());
    }
    let ended_raw = form.ended_at.trim();
    let started_at = ctx
        .parse_datetime_local_input(started_raw)
        .ok_or_else(|| "invalid start".to_string())?;
    let ended_at = if ended_raw.is_empty() {
        None
    } else {
        Some(
            ctx.parse_datetime_local_input(ended_raw)
                .ok_or_else(|| "invalid end".to_string())?,
        )
    };
    Ok(AttendanceInput {
        user_id: form.user_id,
        started_at,
        ended_at,
    })
}

async fn edit_error_response(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &AttendanceForm,
    user_display: &str,
    error: String,
) -> Response {
    let page = AttendanceEditModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(AttendanceEditPostRouteTag::new(id), &q.form_name()),
        user_id: form.user_id,
        user_display: user_display.to_string(),
        started_at: form.started_at.clone(),
        ended_at: form.ended_at.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

fn coord_text(value: Option<f64>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn format_optional_datetime(
    ctx: &AuthContext,
    dt: Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    dt.map(|dt| ctx.format_datetime(dt).into_string())
        .unwrap_or_default()
}

async fn punch_bar(state: &HrState, user_id: i64) -> AttendancePunchBar {
    let geofence = load_attendance_zone(&state.db)
        .await
        .ok()
        .flatten()
        .is_some();
    match punch_subject(&state.db, user_id).await {
        Ok(subject) if !subject.is_employee => AttendancePunchBar {
            punch_in_enabled: false,
            punch_out_enabled: false,
            note: "employee is required".to_string(),
            geofence,
        },
        Ok(subject) => {
            let availability = punch_availability(subject.already_in, subject.employed_long_enough);
            AttendancePunchBar {
                punch_in_enabled: availability.punch_in_enabled,
                punch_out_enabled: availability.punch_out_enabled,
                note: availability.note,
                geofence,
            }
        }
        Err(note) => AttendancePunchBar {
            punch_in_enabled: false,
            punch_out_enabled: false,
            note,
            geofence,
        },
    }
}

/// Superuser punches the filtered user. An employee punches their own user id.
fn punch_user_id(
    is_superuser: bool,
    self_id: i64,
    filter_raw: Option<&str>,
) -> Result<i64, String> {
    if is_superuser {
        parse_user_id(filter_raw).ok_or_else(|| "select an employee".to_string())
    } else {
        Ok(self_id)
    }
}

fn current_list_uri(htmx: &Htmx) -> Uri {
    htmx.current_url
        .as_deref()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or_else(|| {
            AttendanceListRouteTag
                .path()
                .parse()
                .unwrap_or_else(|_| Uri::from_static("/dashboard/hr/attendances"))
        })
}

fn scope_attendance<E>(
    query: Select<E>,
    is_superuser: bool,
    own_employee: bool,
    self_id: i64,
) -> Select<E>
where
    E: EntityTrait,
    E::Column: AttendanceColumns,
{
    if is_superuser {
        query
    } else if own_employee {
        query.filter(E::Column::col_user_id().eq(self_id))
    } else {
        query.filter(sea_orm::sea_query::Expr::cust("1 = 0"))
    }
}

async fn find_attendance_scoped(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    ctx: &AuthContext,
) -> Option<Record> {
    let is_superuser = Superuser::matches(&ctx.role);
    let own_employee = !is_superuser && employee_for_user(db, ctx.user.id).await.is_some();
    match db.get_database_backend() {
        DatabaseBackend::Postgres => {
            find_attendance_on::<postgres::Entity>(db, id, is_superuser, own_employee, ctx.user.id)
                .await
        }
        _ => {
            find_attendance_on::<sqlite::Entity>(db, id, is_superuser, own_employee, ctx.user.id)
                .await
        }
    }
}

async fn find_attendance_on<E>(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    is_superuser: bool,
    own_employee: bool,
    self_id: i64,
) -> Option<Record>
where
    E: EntityTrait,
    E::Column: AttendanceColumns,
    E::Model: Into<Record> + Send + Sync,
{
    lariv_core::web::opt_or_log(
        scope_attendance(
            E::find().filter(E::Column::col_id().eq(id)),
            is_superuser,
            own_employee,
            self_id,
        )
        .one(db)
        .await,
        "find attendance by id",
    )
    .map(Into::into)
}

#[cfg(test)]
mod tests {
    use super::AttendanceListQuery;
    use lariv_core::html_form::UrlencodedFields;

    #[test]
    fn list_query_reads_the_page_parameter() {
        let q: AttendanceListQuery = UrlencodedFields::parse(b"page=2&page_size=36")
            .unwrap()
            .deserialize()
            .unwrap();
        assert_eq!(q.page.get(), 2);
        assert_eq!(q.page_size.get(), 36);
    }

    #[test]
    fn filter_query_accepts_the_form_field_name() {
        let clauses = lariv_core::list_filters::parse_list_filters(
            "filter.0.field=user_id&filter.0.value=4&filter.0.op=eq",
        );
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].field, "user_id");
        assert_eq!(clauses[0].value(), "4");
    }

    #[test]
    fn punch_target_is_self_for_an_employee_and_the_filter_for_a_superuser() {
        assert_eq!(super::punch_user_id(false, 7, Some("4")).unwrap(), 7);
        assert_eq!(super::punch_user_id(true, 1, Some("4")).unwrap(), 4);
        assert!(super::punch_user_id(true, 1, None).is_err());
        assert!(super::punch_user_id(true, 1, Some("")).is_err());
    }
}
