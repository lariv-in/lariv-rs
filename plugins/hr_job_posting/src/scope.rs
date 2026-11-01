//! Applicant scoping and sorting. Self-service HR roles see only their own applicant row.

use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder, Select};

use lariv_plugin_hr::logic::person::person_display_name;
use lariv_plugin_hr::scope::scope_by_user;
use lariv_plugin_users::state::AuthContext;

use crate::entities::applicant::{self, Entity as ApplicantEntity};

pub fn scope_applicants(
    query: Select<ApplicantEntity>,
    auth: &AuthContext,
) -> Select<ApplicantEntity> {
    scope_by_user(query, auth, applicant::Column::UserId)
}

pub async fn find_applicant_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<applicant::Model> {
    lariv_core::web::opt_or_log(
        scope_applicants(ApplicantEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub fn apply_applicant_sort(
    query: Select<ApplicantEntity>,
    sort: Option<&str>,
) -> Select<ApplicantEntity> {
    let sort = sort.unwrap_or("").trim();
    let desc = sort
        .split_whitespace()
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case("DESC"));
    let key = sort.split_whitespace().next().unwrap_or("");
    let column = if key.eq_ignore_ascii_case("Name") {
        applicant::Column::Name
    } else if key.eq_ignore_ascii_case("Mobile") {
        applicant::Column::Mobile
    } else if key.eq_ignore_ascii_case("Email") {
        applicant::Column::Email
    } else {
        return query.order_by_desc(applicant::Column::Id);
    };
    if desc {
        query.order_by_desc(column)
    } else {
        query.order_by_asc(column)
    }
}

pub fn applicant_display_name(applicant: &applicant::Model) -> String {
    person_display_name(&applicant.name, applicant.id, "Applicant")
}
