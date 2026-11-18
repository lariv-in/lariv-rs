use axum::{extract::Query, http::Uri};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::Condition;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

use lariv_core::components::picker::respond_picker_select;
use lariv_core::components::{FilterClauseView, ObjectList, sort_desc, sort_key};
use lariv_core::http::Cap;
use lariv_core::list_filters::{FilterField, apply_filters, clauses_from_query, like_column};
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, path_and_query};
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    entities::bank::{self, Entity as BankEntity},
    keys::{MICR_SELECT_MODAL, MICR_SELECT_TABLE},
    state::BankAccountsState,
    templates::MicrSelectPage,
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct MicrListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct MicrSelectQuery {
    #[serde(flatten)]
    pub filter: MicrListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn micr_fields() -> Vec<FilterField> {
    vec![like_column("micr", "MICR", bank::Column::Micr)]
}

fn apply_micr_sort(
    query: sea_orm::Select<BankEntity>,
    sort: Option<&str>,
) -> sea_orm::Select<BankEntity> {
    let sort = sort.unwrap_or("").trim();
    if sort_key(sort).eq_ignore_ascii_case("MICR") && sort_desc(sort) {
        query.order_by_desc(bank::Column::Micr)
    } else {
        query.order_by_asc(bank::Column::Micr)
    }
}

async fn load_micr_codes(
    db: &sea_orm::DatabaseConnection,
    q: &MicrListQuery,
    page_size: u32,
    condition: Condition,
) -> ObjectList<String> {
    let query = apply_micr_sort(
        scope_allowed::<super::super::routes::BankAccountsView, _>(BankEntity::find())
            .select_only()
            .column(bank::Column::Micr)
            .filter(bank::Column::Micr.is_not_null())
            .filter(bank::Column::Micr.ne(""))
            .filter(condition)
            .group_by(bank::Column::Micr),
        q.sort.as_deref(),
    );
    let page = q.page.get();
    let paginator = query
        .into_tuple::<Option<String>>()
        .paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let codes = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .filter(|code| !code.trim().is_empty())
        .collect();
    ObjectList::from_page(codes, page, page_size, total)
}

pub async fn select(
    Cap(state): Cap<BankAccountsState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<MicrSelectQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""), Some("micr"));
    let fields = micr_fields();
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let codes = load_micr_codes(
        &state.db,
        &q.filter,
        q.filter.page_size.get(),
        applied.condition,
    )
    .await;
    let page = MicrSelectPage {
        codes,
        fields,
        filters: FilterClauseView::from_clauses(&clauses, &applied.errors),
        filter_errors: applied.errors,
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "MicrCode".into()),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select(&htmx, MICR_SELECT_TABLE, MICR_SELECT_MODAL, &page)
}
