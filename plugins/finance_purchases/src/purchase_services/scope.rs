use lariv_core::db::trigram::ci_contains;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, Select};

use crate::purchase_services::entities::service::{self, Entity as ServiceEntity};

pub fn apply_service_filters(
    mut query: Select<ServiceEntity>,
    name: Option<&str>,
    reference: Option<&str>,
) -> Select<ServiceEntity> {
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        query = query.filter(ci_contains(service::Column::Name, n));
    }
    if let Some(r) = reference.filter(|s| !s.is_empty()) {
        query = query.filter(ci_contains(service::Column::Reference, r));
    }
    query
}

pub async fn find_service_scoped(db: &DatabaseConnection, id: i64) -> Option<service::Model> {
    let query =
        scope_allowed::<crate::routes::FinancePurchasesView, _>(ServiceEntity::find_by_id(id));
    lariv_core::web::opt_or_log(query.one(db).await, "find service scoped")
}
