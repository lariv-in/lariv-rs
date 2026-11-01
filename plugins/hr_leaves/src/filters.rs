//! Catalog for leave table filters.
use sea_orm::ColumnTrait;

use sea_orm::sea_query::{Condition, Query as SeaQuery};

use lariv_core::list_filters::{
    FilterField, FilterWidget, choice_column, custom, date_column, foreign_key_column, text_column,
};
use lariv_plugin_users::routes::UsersSelectRouteTag;

use lariv_plugin_hr::entities::leaves::{
    approved_leave, leave_application, leave_type::LeaveType, rejected_leave,
};
use lariv_plugin_hr::logic::leave::{
    FILTER_APPROVED, FILTER_PENDING, FILTER_REJECTED, STATUS_APPROVED, STATUS_PENDING,
    STATUS_REJECTED,
};

fn users_picker_url() -> String {
    UsersSelectRouteTag.url()
}

fn string_choices(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
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
