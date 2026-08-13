use axum::{extract::Query, http::Uri};
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};
use serde::Deserialize;

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList},
    http::Cap,
    picker::respond_picker_select,
    plugins::users::middleware::RequireAuth,
    web::{Htmx, QueryPage},
};

use crate::plugins::finance_accounts::{
    entities::source_doc::{self, Entity as SourceDocEntity},
    keys::{SourceDocSelectModalKey, SourceDocSelectTableKey},
    scope::scope_superuser,
    source_doc_label::resolve_source_doc_display,
    source_doc_registry::SourceDocRegistry,
    state::AccountsState,
    templates::{SourceDocRow, SourceDocSelectPage},
};

use super::util::path_and_query;

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct SourceDocSelectQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub target_input: Option<String>,
}

pub async fn select(
    Cap(state): Cap<AccountsState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<SourceDocSelectQuery>,
) -> maud::Markup {
    let page_num = q.page.get();
    let mut query = scope_superuser(SourceDocEntity::find(), &ctx);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Type DESC") => {
            query.order_by_desc(source_doc::Column::SourceDocType)
        }
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(source_doc::Column::SourceDocType)
        }
        s if s.eq_ignore_ascii_case("Reference DESC") => {
            query.order_by_desc(source_doc::Column::SourceDocId)
        }
        s if s.eq_ignore_ascii_case("Reference ASC") || s.eq_ignore_ascii_case("Reference") => {
            query.order_by_asc(source_doc::Column::SourceDocId)
        }
        _ => query.order_by_desc(source_doc::Column::Id),
    };
    let paginator = query.paginate(&state.db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for d in models {
        let display = resolve_source_doc_display(&state.db, &source_docs, d.id).await;
        rows.push(SourceDocRow {
            id: d.id,
            source_doc_type: display.type_label.clone(),
            source_doc_id: d.source_doc_id,
            label: display.summary_label(),
        });
    }
    let docs = ObjectList::from_page(rows, page_num, PAGE_SIZE, total);
    let page = SourceDocSelectPage {
        docs,
        target_input: q.target_input.unwrap_or_else(|| "SourceDocID".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    respond_picker_select::<SourceDocSelectTableKey, SourceDocSelectModalKey, _>(&htmx, &page)
}
