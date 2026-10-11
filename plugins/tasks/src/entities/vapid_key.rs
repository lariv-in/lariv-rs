use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

/// One persisted VAPID key pair, created when `[tasks]` does not set one.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "task_vapid_keys")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Always `1`. The unique index keeps a single row.
    #[sea_orm(unique)]
    pub singleton: i32,
    pub public_key: String,
    pub private_key: String,
    pub subject: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
