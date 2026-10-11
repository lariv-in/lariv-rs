use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

/// Browser Web Push subscription for one user device.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "task_push_subscriptions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub user_id: i64,
    #[sea_orm(unique)]
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::UserId",
        to = "lariv_plugin_users::entities::user::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}
