//! Catalogs for HR table and picker filters.

use sea_orm::sea_query::{Condition, Query as SeaQuery};
use sea_orm::{ColumnTrait, DatabaseBackend, DatabaseConnection};

use lariv_core::components::FilterClauseView;
use lariv_core::list_filters::{
    FilterClause, FilterField, FilterRowError, FilterWidget, apply_filters, choice_column, custom,
    date_column, datetime_column, foreign_key_column, parse_list_filters, text_column,
};
use lariv_plugin_users::routes::UsersSelectRouteTag;

use crate::entities::{
    applicant, attendance, employee, ex_employee, holiday, job_form,
    leaves::{approved_leave, leave_application, leave_type::LeaveType, rejected_leave},
    overtime::{approved_overtime, overtime_application},
};
use crate::logic::attendance::user_display_label;
use crate::logic::leave::{
    FILTER_APPROVED, FILTER_PENDING, FILTER_REJECTED, STATUS_APPROVED, STATUS_PENDING,
    STATUS_REJECTED,
};

pub struct PreparedFilters {
    pub fields: Vec<FilterField>,
    pub clauses: Vec<FilterClause>,
    pub filters: Vec<FilterClauseView>,
    pub errors: Vec<FilterRowError>,
    pub condition: Condition,
}

pub fn prepare(query: &str, fields: Vec<FilterField>, backend: DatabaseBackend) -> PreparedFilters {
    let clauses = parse_list_filters(query);
    let applied = apply_filters(backend, &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    PreparedFilters {
        fields,
        clauses,
        filters,
        errors: applied.errors,
        condition: applied.condition,
    }
}

pub async fn fill_user_fk(db: &DatabaseConnection, views: &mut [FilterClauseView], key: &str) {
    for view in views.iter_mut() {
        if view.clause.field != key {
            continue;
        }
        let Ok(id) = view.clause.value().parse::<i64>() else {
            continue;
        };
        view.fk_display = user_display_label(db, id).await;
    }
}

fn users_picker_url() -> String {
    UsersSelectRouteTag.url()
}

fn string_choices(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

pub fn applicant_fields() -> Vec<FilterField> {
    vec![
        text_column("name", "Name", applicant::Column::Name),
        text_column("email", "Email", applicant::Column::Email),
    ]
}

pub fn employee_fields() -> Vec<FilterField> {
    vec![
        text_column("name", "Name", employee::Column::Name),
        text_column("email", "Email", employee::Column::Email),
    ]
}

pub fn ex_employee_fields() -> Vec<FilterField> {
    vec![
        text_column("name", "Name", ex_employee::Column::Name),
        text_column("email", "Email", ex_employee::Column::Email),
    ]
}

pub fn holiday_fields() -> Vec<FilterField> {
    vec![
        text_column("title", "Title", holiday::Column::Title),
        date_column("date", "Date", holiday::Column::Date),
    ]
}

pub fn job_form_fields() -> Vec<FilterField> {
    vec![text_column(
        "title",
        "Job title",
        job_form::Column::JobTitle,
    )]
}

pub fn attendance_fields(include_user: bool, tz: &str) -> Vec<FilterField> {
    let mut fields = Vec::new();
    if include_user {
        fields.push(foreign_key_column(
            "user_id",
            "User",
            attendance::Column::UserId,
            users_picker_url(),
            "Any user…",
        ));
    }
    fields.push(datetime_column(
        "started_at",
        "Start",
        attendance::Column::StartedAt,
        tz,
    ));
    fields.push(datetime_column(
        "ended_at",
        "End",
        attendance::Column::EndedAt,
        tz,
    ));
    fields
}

pub fn leave_fields(include_applicant: bool, include_status: bool) -> Vec<FilterField> {
    let mut fields = Vec::new();
    if include_applicant {
        fields.push(foreign_key_column(
            "applied_by_id",
            "Applied by",
            leave_application::Column::AppliedById,
            users_picker_url(),
            "Any user…",
        ));
    }
    fields.push(date_column("date", "Date", leave_application::Column::Date));
    fields.push(choice_column(
        "leave_type",
        "Type",
        leave_application::Column::LeaveType,
        string_choices(LeaveType::choices()),
    ));
    if include_status {
        fields.push(leave_status_field());
    }
    fields.push(text_column(
        "reason",
        "Reason",
        leave_application::Column::Reason,
    ));
    fields
}

fn leave_status_choices() -> Vec<(String, String)> {
    vec![
        (FILTER_PENDING.into(), STATUS_PENDING.into()),
        (FILTER_APPROVED.into(), STATUS_APPROVED.into()),
        (FILTER_REJECTED.into(), STATUS_REJECTED.into()),
    ]
}

fn leave_status_field() -> FilterField {
    let listed = leave_status_choices();
    custom(
        "status",
        "Status",
        FilterWidget::Choice,
        listed.clone(),
        false,
        move |clause, _| {
            let raw = clause.value().to_string();
            if !listed.iter().any(|(key, _)| key == &raw) {
                return Err(format!("unknown value: {raw}"));
            }
            leave_status_condition(&raw).ok_or_else(|| format!("unknown value: {raw}"))
        },
    )
}

pub fn leave_status_condition(status: &str) -> Option<Condition> {
    let approved = SeaQuery::select()
        .column(approved_leave::Column::LeaveApplicationId)
        .from(approved_leave::Entity)
        .to_owned();
    let rejected = SeaQuery::select()
        .column(rejected_leave::Column::LeaveApplicationId)
        .from(rejected_leave::Entity)
        .to_owned();
    match status {
        FILTER_APPROVED => {
            Some(Condition::all().add(leave_application::Column::Id.in_subquery(approved)))
        }
        FILTER_REJECTED => Some(
            Condition::all()
                .add(leave_application::Column::Id.in_subquery(rejected))
                .add(leave_application::Column::Id.not_in_subquery(approved)),
        ),
        FILTER_PENDING => Some(
            Condition::all()
                .add(leave_application::Column::Id.not_in_subquery(approved))
                .add(leave_application::Column::Id.not_in_subquery(rejected)),
        ),
        _ => None,
    }
}

pub fn overtime_application_fields(include_user: bool, tz: &str) -> Vec<FilterField> {
    let mut fields = Vec::new();
    if include_user {
        fields.push(foreign_key_column(
            "user_id",
            "User",
            overtime_application::Column::UserId,
            users_picker_url(),
            "Any user…",
        ));
    }
    fields.push(datetime_column(
        "start_time",
        "Start",
        overtime_application::Column::StartTime,
        tz,
    ));
    fields.push(datetime_column(
        "end_time",
        "End",
        overtime_application::Column::EndTime,
        tz,
    ));
    fields.push(text_column(
        "reason",
        "Reason",
        overtime_application::Column::Reason,
    ));
    fields
}

pub fn approved_overtime_fields(include_user: bool, tz: &str) -> Vec<FilterField> {
    let mut fields = Vec::new();
    if include_user {
        fields.push(foreign_key_column(
            "user_id",
            "User",
            approved_overtime::Column::UserId,
            users_picker_url(),
            "Any user…",
        ));
    }
    fields.push(datetime_column(
        "start_time",
        "Start",
        approved_overtime::Column::StartTime,
        tz,
    ));
    fields.push(datetime_column(
        "end_time",
        "End",
        approved_overtime::Column::EndTime,
        tz,
    ));
    fields
}
