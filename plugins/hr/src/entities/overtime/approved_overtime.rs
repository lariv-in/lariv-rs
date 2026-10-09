use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_approved_overtimes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(indexed)]
    pub user_id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    #[sea_orm(indexed)]
    pub approved_by_id: i64,
    pub approved_at: DateTime<Utc>,
    /// At most one approval row per application. Null when the application was removed.
    #[sea_orm(unique)]
    pub overtime_application_id: Option<i64>,
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
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::ApprovedById",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "Restrict"
    )]
    ApprovedBy,
    #[sea_orm(
        belongs_to = "super::overtime_application::Entity",
        from = "Column::OvertimeApplicationId",
        to = "super::overtime_application::Column::Id",
        on_delete = "SetNull"
    )]
    OvertimeApplication,
}

impl Related<super::overtime_application::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OvertimeApplication.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
