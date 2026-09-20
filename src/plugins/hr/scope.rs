use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Select, sea_query::Expr,
};

use crate::plugins::users::state::AuthContext;

use super::entities::{
    applicant::{self, Entity as ApplicantEntity},
    employee::{self, Entity as EmployeeEntity},
    ex_employee::{self, Entity as ExEmployeeEntity},
    probation::{self, Entity as ProbationEntity},
};
use super::logic::person::person_display_name;

pub fn scope_superuser<T>(query: Select<T>, auth: &AuthContext) -> Select<T>
where
    T: sea_orm::EntityTrait,
{
    if auth.user.is_superuser {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

pub async fn find_applicant_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<applicant::Model> {
    crate::web::opt_or_log(
        scope_superuser(ApplicantEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_probation_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<probation::Model> {
    crate::web::opt_or_log(
        scope_superuser(ProbationEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_employee_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<employee::Model> {
    crate::web::opt_or_log(
        scope_superuser(EmployeeEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_ex_employee_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<ex_employee::Model> {
    crate::web::opt_or_log(
        scope_superuser(ExEmployeeEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

fn sort_key(sort: &str) -> &str {
    sort.trim().split_whitespace().next().unwrap_or("")
}

fn sort_desc(sort: &str) -> bool {
    sort.split_whitespace()
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case("DESC"))
}

macro_rules! apply_person_filters {
    ($query:expr, $col:ident, $name:expr, $email:expr) => {{
        let mut query = $query;
        if let Some(n) = $name.filter(|s| !s.is_empty()) {
            query = query.filter($col::Column::Name.contains(n));
        }
        if let Some(e) = $email.filter(|s| !s.is_empty()) {
            query = query.filter($col::Column::Email.contains(e));
        }
        query
    }};
}

macro_rules! apply_person_sort {
    ($query:expr, $col:ident, $sort:expr) => {{
        let sort = $sort.unwrap_or("").trim();
        let desc = sort_desc(sort);
        match sort_key(sort) {
            s if s.eq_ignore_ascii_case("Name") => {
                if desc {
                    $query.order_by_desc($col::Column::Name)
                } else {
                    $query.order_by_asc($col::Column::Name)
                }
            }
            s if s.eq_ignore_ascii_case("Mobile") => {
                if desc {
                    $query.order_by_desc($col::Column::Mobile)
                } else {
                    $query.order_by_asc($col::Column::Mobile)
                }
            }
            s if s.eq_ignore_ascii_case("Email") => {
                if desc {
                    $query.order_by_desc($col::Column::Email)
                } else {
                    $query.order_by_asc($col::Column::Email)
                }
            }
            _ => $query.order_by_desc($col::Column::Id),
        }
    }};
}

pub fn apply_applicant_filters(
    query: Select<ApplicantEntity>,
    name: Option<&str>,
    email: Option<&str>,
) -> Select<ApplicantEntity> {
    apply_person_filters!(query, applicant, name, email)
}

pub fn apply_applicant_sort(
    query: Select<ApplicantEntity>,
    sort: Option<&str>,
) -> Select<ApplicantEntity> {
    apply_person_sort!(query, applicant, sort)
}

pub fn apply_probation_filters(
    query: Select<ProbationEntity>,
    name: Option<&str>,
    email: Option<&str>,
) -> Select<ProbationEntity> {
    apply_person_filters!(query, probation, name, email)
}

pub fn apply_probation_sort(
    query: Select<ProbationEntity>,
    sort: Option<&str>,
) -> Select<ProbationEntity> {
    apply_person_sort!(query, probation, sort)
}

pub fn apply_employee_filters(
    query: Select<EmployeeEntity>,
    name: Option<&str>,
    email: Option<&str>,
) -> Select<EmployeeEntity> {
    apply_person_filters!(query, employee, name, email)
}

pub fn apply_employee_sort(
    query: Select<EmployeeEntity>,
    sort: Option<&str>,
) -> Select<EmployeeEntity> {
    apply_person_sort!(query, employee, sort)
}

pub fn apply_ex_employee_filters(
    query: Select<ExEmployeeEntity>,
    name: Option<&str>,
    email: Option<&str>,
) -> Select<ExEmployeeEntity> {
    apply_person_filters!(query, ex_employee, name, email)
}

pub fn apply_ex_employee_sort(
    query: Select<ExEmployeeEntity>,
    sort: Option<&str>,
) -> Select<ExEmployeeEntity> {
    apply_person_sort!(query, ex_employee, sort)
}

pub fn applicant_display_name(applicant: &applicant::Model) -> String {
    person_display_name(&applicant.name, applicant.id, "Applicant")
}

pub fn probation_display_name(probation: &probation::Model) -> String {
    person_display_name(&probation.name, probation.id, "Probation")
}

pub fn employee_display_name(employee: &employee::Model) -> String {
    person_display_name(&employee.name, employee.id, "Employee")
}

pub fn ex_employee_display_name(ex_employee: &ex_employee::Model) -> String {
    person_display_name(&ex_employee.name, ex_employee.id, "Ex-employee")
}

pub fn format_timestamp(dt: chrono::DateTime<chrono::Utc>, tz: &str) -> String {
    crate::datetime::DatetimeLabel::short(dt, tz).into_string()
}
