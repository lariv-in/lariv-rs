//! HTTP handler for one user's audit log.

use axum::extract::{Path, Query};
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use sea_orm::sea_query::Condition;
use sea_orm::{ColumnTrait, EntityTrait};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, html_built_page_with_slots};
use lariv_plugin_logging::entities::log::Column as LogColumn;
use lariv_plugin_logging::handlers::{LogListQuery, load_log_list};
use lariv_plugin_logging::keys::LOG_TABLE;
use lariv_plugin_logging::templates::LogListPage;
use lariv_plugin_users::entities::user::Entity as UserEntity;
use lariv_plugin_users::middleware::{RequireAuth, can_change_user_password};

use crate::routes::UserLogListRouteTag;
use crate::state::UserLogState;
use crate::templates::UserLogPage;

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

/// HTTP handler: audit log rows written by one user.
pub async fn list(
    Cap(state): Cap<UserLogState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<LogListQuery>,
) -> Response {
    let user = match UserEntity::find_by_id(id).one(&state.db).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(error = %error, user_id = id, "load user for audit log");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let data = load_log_list(
        &state.db,
        &q,
        uri.query().unwrap_or(""),
        &ctx.timezone,
        Condition::all().add(LogColumn::UserId.eq(id)),
        &["user_id"],
    )
    .await;
    let page = UserLogPage {
        user_id: user.id,
        user_name: user.name,
        show_change_password: can_change_user_password(&ctx, user.id),
        logs: LogListPage {
            logs: data.logs,
            fields: data.fields,
            filters: data.filters,
            filter_errors: data.filter_errors,
            sort: data.sort,
            path_and_query: path_and_query(&uri),
            page_size: data.page_size,
            filter_form_url: UserLogListRouteTag::new(id).url(),
        },
    };
    if htmx.targets(LOG_TABLE) {
        return page.logs.render_table().into_response();
    }
    if htmx.wants_main_content() {
        let markup: maud::Markup = page.render_main().into();
        return markup.into_response();
    }
    if htmx.wants_app_layout() {
        let markup: maud::Markup = page.render_pane().into();
        return markup.into_response();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}
