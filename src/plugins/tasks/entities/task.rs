use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub title: String,
    pub description: String,
    pub assigned_to_id: i64,
    pub status_id: i64,
    pub priority: i32,
    pub due_datetime: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::AssignedToId",
        to = "crate::plugins::users::entities::user::Column::Id"
    )]
    AssignedTo,
    #[sea_orm(
        belongs_to = "super::task_status::Entity",
        from = "Column::StatusId",
        to = "super::task_status::Column::Id"
    )]
    Status,
    #[sea_orm(has_many = "super::task_log::Entity")]
    Logs,
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AssignedTo.def()
    }
}

impl Related<super::task_status::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Status.def()
    }
}

impl Related<super::task_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Logs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
