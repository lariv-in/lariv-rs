use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{Htmx, QueryPage, html_built_page_or_app_layout},
};

use crate::plugins::crm::{
    entities::{
        converted_lead::{self, Entity as ConvertedLeadEntity},
        failed_lead::{self, Entity as FailedLeadEntity},
        lead_timeline::{self, Entity as LeadTimelineEntity},
    },
    keys::LeadTimelineKey,
    routes::LeadDefaultRouteTag,
    scope::{find_lead_scoped, lead_display_name},
    state::CrmState,
    templates::{LeadTimelinePage, LeadTimelineRow},
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct LeadTimelineQuery {
    #[serde(default)]
    pub page: QueryPage,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub async fn page(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<LeadTimelineQuery>,
) -> Response {
    let Some(lead) = find_lead_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    };
    let converted_id = ConvertedLeadEntity::find()
        .filter(converted_lead::Column::LeadId.eq(lead.id))
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|c| c.id)
        .unwrap_or(0);
    let failed_id = FailedLeadEntity::find()
        .filter(failed_lead::Column::LeadId.eq(lead.id))
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|f| f.id)
        .unwrap_or(0);
    let page_num = q.page.get();
    let paginator = LeadTimelineEntity::find()
        .filter(lead_timeline::Column::LeadId.eq(lead.id))
        .order_by_desc(lead_timeline::Column::CreatedAt)
        .order_by_desc(lead_timeline::Column::Id)
        .paginate(&state.db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|m| LeadTimelineRow {
            created_at: ctx.format_datetime(m.created_at).into_string(),
            content: m.content,
        })
        .collect();
    let page = LeadTimelinePage {
        lead_id: lead.id,
        converted_id,
        failed_id,
        display_name: lead_display_name(&state.db, &lead).await,
        items: ObjectList::from_page(rows, page_num, PAGE_SIZE, total),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<LeadTimelineKey>() {
        return page.render_timeline().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}
