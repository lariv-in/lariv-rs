use lariv_core::db::trigram::ci_contains;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, Select};

use crate::entities::product::{self, Entity as ProductEntity};

pub fn apply_product_filters(
    mut query: Select<ProductEntity>,
    name: Option<&str>,
    reference: Option<&str>,
) -> Select<ProductEntity> {
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(ci_contains(product::Column::Name, n));
    }
    if let Some(r) = reference.filter(|s| !s.is_empty()) {
        query = query.filter(ci_contains(product::Column::Reference, r));
    }
    query
}

pub async fn find_product_scoped(db: &DatabaseConnection, id: i64) -> Option<product::Model> {
    let query =
        scope_allowed::<super::routes::FinanceProductsView, _>(ProductEntity::find_by_id(id));
    lariv_core::web::opt_or_log(query.one(db).await, "find product scoped")
}
