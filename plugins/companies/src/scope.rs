use lariv_core::components::{sort_desc, sort_key};
use lariv_plugin_users::role_authorization::{current_auth, scope_allowed};
use lariv_plugin_users::roles::Superuser;
use sea_orm::sea_query::{Query, SelectStatement};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Select};

use super::entities::company::{self, Entity as CompanyEntity};
use super::entities::company_membership;
use super::routes::CompaniesView;

pub async fn find_company_scoped(db: &DatabaseConnection, id: i64) -> Option<company::Model> {
    lariv_core::web::opt_or_log(
        visible_companies(scope_allowed::<CompaniesView, _>(
            CompanyEntity::find_by_id(id),
        ))
        .one(db)
        .await,
        "find company scoped",
    )
}

/// Hide companies the signed-in user does not belong to.
///
/// The companies RLS policy already says this. A Postgres superuser login skips
/// that policy, so the list, picker, and detail queries repeat it here.
/// Application superusers still see every company. No signed-in user means the
/// query is unchanged.
pub fn visible_companies(query: Select<CompanyEntity>) -> Select<CompanyEntity> {
    let Some(auth) = current_auth() else {
        return query;
    };
    if Superuser::matches(&auth.role) {
        return query;
    }
    query.filter(company::Column::Id.in_subquery(member_company_ids(auth.user.id)))
}

fn member_company_ids(user_id: i64) -> SelectStatement {
    Query::select()
        .column(company_membership::Column::CompanyId)
        .from(company_membership::Entity)
        .and_where(company_membership::Column::UserId.eq(user_id))
        .to_owned()
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
