use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Select};

use crate::plugins::users::state::AuthContext;

use crate::plugins::finance_common::is_superuser;

use crate::plugins::finance_products::entities::product::{self, Entity as ProductEntity};

pub fn scope_products(query: Select<ProductEntity>, auth: &AuthContext) -> Select<ProductEntity> {
    if is_superuser(auth) {
        return query;
    }
    query.filter(product::Column::Id.eq(-1))
}

pub fn apply_product_filters(
    mut query: Select<ProductEntity>,
    name: Option<&str>,
    reference: Option<&str>,
) -> Select<ProductEntity> {
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(product::Column::Name.contains(n));
    }
    if let Some(r) = reference.filter(|s| !s.is_empty()) {
        query = query.filter(product::Column::Reference.contains(r));
    }
    query.order_by_desc(product::Column::UpdatedAt)
}

pub async fn find_product_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<product::Model> {
    let query = ProductEntity::find_by_id(id);
    scope_products(query, auth).one(db).await.ok().flatten()
}
