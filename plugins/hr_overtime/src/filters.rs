//! Catalogs for overtime table filters.

use sea_orm::DatabaseConnection;

use lariv_core::components::FilterClauseView;
use lariv_core::list_filters::{FilterField, datetime_column, foreign_key_column, text_column};
use lariv_plugin_hr::logic::user::user_display_label;
use lariv_plugin_users::routes::UsersSelectRouteTag;

use crate::entities::overtime::{approved_overtime, overtime_application};

pub use lariv_plugin_hr::filters::{PreparedFilters, prepare};

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
