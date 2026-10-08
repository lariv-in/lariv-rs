use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::leave_type::LeaveType;

/// Append-only balance of approved leave, one row per status change.
///
/// Approving a leave posts `amount = -1`. Leaving the approved state posts `amount = 1`.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_leave_journal")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(indexed)]
    pub user_id: i64,
    pub datetime: DateTime<Utc>,
    pub leave_type: LeaveType,
    pub amount: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::UserId",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
