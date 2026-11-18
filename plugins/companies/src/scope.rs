use lariv_core::components::{sort_desc, sort_key};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder, Select};

use super::entities::company::{self, Entity as CompanyEntity};
use super::routes::CompaniesView;

pub async fn find_company_scoped(db: &DatabaseConnection, id: i64) -> Option<company::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<CompaniesView, _>(CompanyEntity::find_by_id(id))
            .one(db)
            .await,
        "find company scoped",
    )
}

pub fn apply_company_sort(
    query: Select<CompanyEntity>,
    sort: Option<&str>,
) -> Select<CompanyEntity> {
    let sort = sort.unwrap_or("").trim();
    match sort_key(sort) {
        s if s.eq_ignore_ascii_case("Name") => {
            if sort_desc(sort) {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        _ => query.order_by_desc(company::Column::Id),
    }
}

pub async fn company_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    lariv_core::web::opt_or_log(CompanyEntity::find_by_id(id).one(db).await, "find by id")
        .map(|c| c.name)
        .unwrap_or_default()
}
