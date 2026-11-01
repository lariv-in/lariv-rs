//! Catalog for attendance table filters.

use lariv_core::list_filters::{FilterField, datetime_column, foreign_key_column};
use lariv_plugin_users::routes::UsersSelectRouteTag;

use crate::entities::attendance;

pub fn attendance_fields(include_user: bool, tz: &str) -> Vec<FilterField> {
    let mut fields = Vec::new();
    if include_user {
        fields.push(foreign_key_column(
            "user_id",
            "User",
            attendance::sqlite::Column::UserId,
            UsersSelectRouteTag.url(),
            "Any user…",
        ));
    }
    fields.push(datetime_column(
        "started_at",
        "Start",
        attendance::sqlite::Column::StartedAt,
        tz,
    ));
    fields.push(datetime_column(
        "ended_at",
        "End",
        attendance::sqlite::Column::EndedAt,
        tz,
    ));
    fields
}
