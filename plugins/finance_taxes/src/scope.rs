use lariv_core::db::trigram::ci_contains;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Select};

use crate::entities::tax::{self, Entity as TaxEntity};
use crate::forms::tax_type_label;

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

pub fn apply_tax_filters(
    mut query: Select<TaxEntity>,
    name: Option<&str>,
    tax_type: Option<&str>,
) -> Select<TaxEntity> {
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(ci_contains(tax::Column::Name, n));
    }
    if let Some(kind) = tax_type
        .filter(|s| !s.is_empty())
        .and_then(crate::entities::TaxKind::parse)
    {
        query = query.filter(tax::Column::TaxType.eq(kind));
    }
    query
}

pub async fn find_tax_scoped(db: &DatabaseConnection, id: i64) -> Option<tax::Model> {
    let query = scope_allowed::<super::routes::FinanceTaxesView, _>(TaxEntity::find_by_id(id));
    lariv_core::web::opt_or_log(query.one(db).await, "find tax scoped")
}

pub async fn model_to_row(_db: &DatabaseConnection, t: tax::Model) -> crate::templates::TaxRow {
    crate::templates::TaxRow {
        id: t.id,
        name: t.name,
        tax_type: tax_type_label(&t.tax_type),
        percentage: t.percentage.normalize().to_string(),
    }
}
