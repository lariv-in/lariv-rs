use axum::{extract::Query, http::Uri};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::Condition;
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter};

use lariv_core::components::picker::respond_picker_select;
use lariv_core::components::{FilterClauseView, ObjectList};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    FilterField, apply_filters, clauses_from_query, like_column, text_column,
};
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, path_and_query};
use lariv_plugin_users::{middleware::RequireAuth, state::AuthContext};

use crate::{
    entities::bank::{self, Entity as BankEntity},
    keys::{BANK_SELECT_MODAL, BANK_SELECT_TABLE},
    scope::apply_bank_sort,
    state::BankAccountsState,
    templates::{BankRow, BankSelectPage},
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct BankListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BankSelectQuery {
    #[serde(flatten)]
    pub filter: BankListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn bank_fields() -> Vec<FilterField> {
    vec![
        like_column("ifsc", "IFSC", bank::Column::Ifsc),
        text_column("bank_name", "Bank", bank::Column::BankName),
        text_column("branch_name", "Branch", bank::Column::BranchName),
        text_column("city", "City", bank::Column::City),
    ]
}

async fn query_banks(
    db: &sea_orm::DatabaseConnection,
    q: &BankListQuery,
    _auth: &AuthContext,
    page_size: u32,
    condition: Condition,
) -> (Vec<bank::Model>, u32, u64) {
    let mut query = scope_allowed::<super::super::routes::BankAccountsView, _>(BankEntity::find());
    query = query.filter(condition);
    query = apply_bank_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

fn model_to_row(b: bank::Model) -> BankRow {
    let display = b.display_label();
    BankRow {
        ifsc: b.ifsc,
        bank_name: b.bank_name.unwrap_or_default(),
        branch_name: b.branch_name.unwrap_or_default(),
        city: b.city.unwrap_or_default(),
        display,
    }
}

async fn load_bank_rows(
    db: &sea_orm::DatabaseConnection,
    q: &BankListQuery,
    auth: &AuthContext,
    page_size: u32,
    condition: Condition,
) -> ObjectList<BankRow> {
    let (models, page, total) = query_banks(db, q, auth, page_size, condition).await;
    let rows = models.into_iter().map(model_to_row).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn select(
    Cap(state): Cap<BankAccountsState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<BankSelectQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""), Some("ifsc"));
    let fields = bank_fields();
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let banks = load_bank_rows(
        &state.db,
        &q.filter,
        &ctx,
        q.filter.page_size.get(),
        applied.condition,
    )
    .await;
    let page = BankSelectPage {
        banks,
        fields,
        filters: FilterClauseView::from_clauses(&clauses, &applied.errors),
        filter_errors: applied.errors,
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "IfscCode".into()),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select(&htmx, BANK_SELECT_TABLE, BANK_SELECT_MODAL, &page)
}
