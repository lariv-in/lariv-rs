use crate::plugins::users::role_authorization::scope_allowed;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Select};

use super::entities::customer::{self, Entity as CustomerEntity};

pub fn apply_customer_filters(
    mut query: Select<CustomerEntity>,
    name: Option<&str>,
    email: Option<&str>,
) -> Select<CustomerEntity> {
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(customer::Column::Name.contains(n));
    }
    if let Some(e) = email.filter(|s| !s.is_empty()) {
        query = query.filter(customer::Column::Email.contains(e));
    }
    query
}

pub async fn find_customer_scoped(db: &DatabaseConnection, id: i64) -> Option<customer::Model> {
    crate::web::opt_or_log(
        scope_allowed::<super::routes::CustomerView, _>(CustomerEntity::find_by_id(id))
            .one(db)
            .await,
        "find customer scoped",
    )
}
