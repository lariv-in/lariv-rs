use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub title: String,
    pub description: Option<String>,
    pub assigned_to_id: i64,
    pub due_date: Option<NaiveDate>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::AssignedToId",
        to = "crate::plugins::users::entities::user::Column::Id"
    )]
    AssignedTo,
    #[sea_orm(has_one = "super::completed_task::Entity")]
    CompletedTask,
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AssignedTo.def()
    }
}

impl Related<super::completed_task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CompletedTask.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
