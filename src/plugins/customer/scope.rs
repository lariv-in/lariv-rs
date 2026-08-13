use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Select, sea_query::Expr};

use crate::plugins::users::state::AuthContext;

use super::entities::customer::{self, Entity as CustomerEntity};

pub fn scope_customers(
    query: Select<CustomerEntity>,
    auth: &AuthContext,
) -> Select<CustomerEntity> {
    if auth.user.is_superuser {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

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

pub async fn find_customer_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<customer::Model> {
    let query = CustomerEntity::find_by_id(id);
    scope_customers(query, auth).one(db).await.ok().flatten()
}
