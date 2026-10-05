use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_rejected_leaves")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// At most one rejection row per application.
    #[sea_orm(unique)]
    pub leave_application_id: i64,
    #[sea_orm(indexed)]
    pub rejected_by_id: i64,
    pub rejected_at: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::leave_application::Entity",
        from = "Column::LeaveApplicationId",
        to = "super::leave_application::Column::Id",
        on_delete = "Cascade"
    )]
    LeaveApplication,
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::RejectedById",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "Restrict"
    )]
    RejectedBy,
}

impl Related<super::leave_application::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LeaveApplication.def()
    }
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RejectedBy.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
