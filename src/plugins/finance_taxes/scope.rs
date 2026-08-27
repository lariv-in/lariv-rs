use sea_orm::{
    sea_query::Expr, ColumnTrait, DatabaseConnection, DbBackend, EntityTrait, FromQueryResult,
    QueryFilter, Select, Statement,
};

use crate::plugins::users::state::AuthContext;

use crate::plugins::finance_common::is_superuser;

use crate::plugins::finance_taxes::entities::tax::{self, Entity as TaxEntity};
use crate::plugins::finance_taxes::forms::tax_type_label;

#[derive(Debug, FromQueryResult)]
struct AccountRow {
    name: String,
    code: i32,
}

pub async fn account_label(db: &DatabaseConnection, account_id: Option<i64>) -> String {
    let Some(id) = account_id.filter(|&id| id > 0) else {
        return "—".to_string();
    };
    let row = crate::web::opt_or_log(
        AccountRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT name, code FROM accounts WHERE id = $1 LIMIT 1",
            [id.into()],
        ))
        .one(db)
        .await,
        "db find one",
    );
    match row {
        Some(r) if r.code != 0 => format!("{} — {}", r.code, r.name),
        Some(r) => format!("{} (#{})", r.name, id),
        None => format!("#{}", id),
    }
}

pub async fn load_taxes_by_ids(
    db: &DatabaseConnection,
    ids: &[i64],
) -> Result<Vec<tax::Model>, sea_orm::DbErr> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    TaxEntity::find()
        .filter(tax::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await
}

pub async fn load_all_taxes(db: &DatabaseConnection) -> Result<Vec<tax::Model>, sea_orm::DbErr> {
    TaxEntity::find().all(db).await
}

pub fn tax_label(t: &tax::Model) -> String {
    if t.name.is_empty() {
        format!("#{}", t.id)
    } else {
        t.name.clone()
    }
}

pub fn scope_taxes(query: Select<TaxEntity>, auth: &AuthContext) -> Select<TaxEntity> {
    if is_superuser(auth) {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

pub fn apply_tax_filters(
    mut query: Select<TaxEntity>,
    name: Option<&str>,
    tax_type: Option<&str>,
) -> Select<TaxEntity> {
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(tax::Column::Name.contains(n));
    }
    if let Some(kind) = tax_type
        .filter(|s| !s.is_empty())
        .and_then(crate::plugins::finance_taxes::entities::TaxKind::parse)
    {
        query = query.filter(tax::Column::TaxType.eq(kind));
    }
    query
}

pub async fn find_tax_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<tax::Model> {
    let query = TaxEntity::find_by_id(id);
    crate::web::opt_or_log(scope_taxes(query, auth).one(db).await, "find tax scoped")
}

pub async fn model_to_row(
    db: &DatabaseConnection,
    t: tax::Model,
) -> crate::plugins::finance_taxes::templates::TaxRow {
    crate::plugins::finance_taxes::templates::TaxRow {
        id: t.id,
        name: t.name,
        tax_type: tax_type_label(&t.tax_type),
        percentage: t.percentage.normalize().to_string(),
        account_label: account_label(db, t.account_id).await,
    }
}
