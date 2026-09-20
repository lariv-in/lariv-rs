use axum::{
    extract::Query,
    http::Uri,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done,
    },
};

use crate::plugins::meets::{
    entities::conference_room::{self, Entity as ConferenceRoomEntity},
    forms::CreateRoomForm,
    handlers::ModalNameQuery,
    keys::{MeetsCreateModalKey, MeetsHubTableKey},
    logic::rooms::{CreateRoomInput, create_room, room_is_live},
    routes::RoomRouteTag,
    state::MeetsState,
    templates::{MeetsCreateModalPage, MeetsHubPage, RoomRow},
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct HubQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default, rename = "Code", alias = "code")]
    pub code: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub async fn hub(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<HubQuery>,
) -> maud::Markup {
    let page_num = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.get();
    let mut query = ConferenceRoomEntity::find();
    if !ctx.user.is_superuser {
        query = query.filter(conference_room::Column::CreatedById.eq(ctx.user.id));
    }
    if let Some(code) = q.code.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        query = query.filter(conference_room::Column::Code.contains(code));
    }
    let sort = q.sort.as_deref().unwrap_or("");
    query = match sort {
        s if s.eq_ignore_ascii_case("Code DESC") => {
            query.order_by_desc(conference_room::Column::Code)
        }
        s if s.eq_ignore_ascii_case("Code ASC") || s.eq_ignore_ascii_case("Code") => {
            query.order_by_asc(conference_room::Column::Code)
        }
        s if s.eq_ignore_ascii_case("CreatedAt ASC") || s.eq_ignore_ascii_case("CreatedAt") => {
            query.order_by_asc(conference_room::Column::CreatedAt)
        }
        _ => query.order_by_desc(conference_room::Column::CreatedAt),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let now = Utc::now();
    let tz = ctx.timezone.as_str();
    let rows: Vec<RoomRow> = models
        .into_iter()
        .map(|r| RoomRow {
            created_at: crate::datetime::DatetimeLabel::short(r.created_at, tz).into_string(),
            live: room_is_live(&r, now),
            joining_allowed: r.joining_allowed,
            code: r.code,
        })
        .collect();
    let page = MeetsHubPage {
        rooms: ObjectList::from_page(rows, page_num, page_size, total),
        filter_code: q.code.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<MeetsHubTableKey>() {
        return page.render_table();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = MeetsCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        anonymous_allowed: false,
        joining_allowed: true,
        start_at: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<MeetsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<CreateRoomForm>,
) -> Response {
    let start_at = if form.start_at.trim().is_empty() {
        None
    } else {
        ctx.parse_datetime_local_input(&form.start_at)
    };
    match create_room(
        &state.db,
        state.config.room_code_length,
        CreateRoomInput {
            created_by_id: ctx.user.id,
            anonymous_allowed: form.anonymous_allowed,
            joining_allowed: form.joining_allowed,
            start_at,
        },
    )
    .await
    {
        Ok(room) => respond_create_modal_done::<MeetsCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &RoomRouteTag::new(room.code).url(),
        ),
        Err(e) => {
            let page = MeetsCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                anonymous_allowed: form.anonymous_allowed,
                joining_allowed: form.joining_allowed,
                start_at: form.start_at,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
