use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::leave_type::LeaveType;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_leave_applications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// User who submitted the application.
    #[sea_orm(indexed)]
    pub applied_by_id: i64,
    #[sea_orm(indexed)]
    pub date: NaiveDate,
    pub reason: String,
    pub leave_type: LeaveType,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::AppliedById",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    AppliedBy,
    #[sea_orm(has_one = "super::approved_leave::Entity")]
    ApprovedLeave,
    #[sea_orm(has_one = "super::rejected_leave::Entity")]
    RejectedLeave,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AppliedBy.def()
    }
}

impl Related<super::approved_leave::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApprovedLeave.def()
    }
}

impl Related<super::rejected_leave::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RejectedLeave.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
