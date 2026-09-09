use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_cron_jobs")]
/// Interval job that opens a new assistant conversation on a duration.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Interval between runs, stored as nanoseconds.
    pub duration: i64,
    #[sea_orm(column_type = "Text")]
    pub prompt: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::cron_job_run::Entity")]
    Runs,
}

impl Related<super::cron_job_run::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Runs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type CronJob = Model;
