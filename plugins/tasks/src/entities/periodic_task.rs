use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A task template that the polling worker materializes on an interval.
///
/// `frequency` is the interval in nanoseconds (`chrono::Duration`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "periodic_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub title: String,
    pub description: String,
    pub assigned_to_id: i64,
    pub status_id: i64,
    pub parent_id: Option<i64>,
    pub priority: i32,
    pub due_datetime: DateTime<Utc>,
    /// Interval between spawned tasks, stored as nanoseconds.
    pub frequency: i64,
    pub last_added_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::AssignedToId",
        to = "lariv_plugin_users::entities::user::Column::Id"
    )]
    AssignedTo,
    #[sea_orm(
        belongs_to = "super::task_status::Entity",
        from = "Column::StatusId",
        to = "super::task_status::Column::Id"
    )]
    Status,
    #[sea_orm(
        belongs_to = "super::task::Entity",
        from = "Column::ParentId",
        to = "super::task::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Parent,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AssignedTo.def()
    }
}

impl Related<super::task_status::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Status.def()
    }
}

impl Related<super::task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
