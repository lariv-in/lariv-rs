use lariv_core::components::{sort_desc, sort_key};
use lariv_plugin_companies::entities::company;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Select,
};

use super::entities::contact::{self, Entity as ContactEntity};

pub async fn find_contact_scoped(db: &DatabaseConnection, id: i64) -> Option<contact::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::ContactsView, _>(ContactEntity::find_by_id(id))
            .one(db)
            .await,
        "find contact scoped",
    )
}

pub fn apply_contact_sort(
    mut query: Select<ContactEntity>,
    sort: Option<&str>,
) -> Select<ContactEntity> {
    let sort = sort.unwrap_or("").trim();
    let key = sort_key(sort);
    if key.eq_ignore_ascii_case("Company") {
        query = query.join(JoinType::LeftJoin, contact::Relation::Company.def());
    }
    let desc = sort_desc(sort);
    match key {
        s if s.eq_ignore_ascii_case("Name") => {
            if desc {
                query.order_by_desc(contact::Column::Name)
            } else {
                query.order_by_asc(contact::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Company") => {
            if desc {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Email") => {
            if desc {
                query.order_by_desc(contact::Column::Email)
            } else {
                query.order_by_asc(contact::Column::Email)
            }
        }
        _ => query.order_by_desc(contact::Column::Id),
    }
}

pub async fn contact_belongs_to_company(
    db: &DatabaseConnection,
    contact_id: i64,
    company_id: i64,
) -> bool {
    lariv_core::web::opt_or_log(
        ContactEntity::find_by_id(contact_id)
            .filter(contact::Column::CompanyId.eq(company_id))
            .one(db)
            .await,
        "find by id",
    )
    .is_some()
}

pub async fn contact_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    lariv_core::web::opt_or_log(ContactEntity::find_by_id(id).one(db).await, "find by id")
        .map(|c| c.display_name())
        .unwrap_or_default()
}
