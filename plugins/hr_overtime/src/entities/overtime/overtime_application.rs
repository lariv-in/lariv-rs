use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_overtime_applications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Employee who submitted the application.
    #[sea_orm(indexed)]
    pub user_id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub reason: Option<String>,
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
    #[sea_orm(has_one = "super::approved_overtime::Entity")]
    ApprovedOvertime,
    #[sea_orm(has_one = "super::rejected_overtime::Entity")]
    RejectedOvertime,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::approved_overtime::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApprovedOvertime.def()
    }
}

impl Related<super::rejected_overtime::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RejectedOvertime.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
