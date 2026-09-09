use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_cron_job_runs")]
/// One firing of a cron job and the conversation it created.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub cron_job_id: i64,
    pub datetime: DateTime<Utc>,
    pub session_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::cron_job::Entity",
        from = "Column::CronJobId",
        to = "super::cron_job::Column::Id",
        on_delete = "Cascade"
    )]
    CronJob,
    #[sea_orm(
        belongs_to = "super::session::Entity",
        from = "Column::SessionId",
        to = "super::session::Column::Id",
        on_delete = "SetNull"
    )]
    Session,
}

impl Related<super::cron_job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CronJob.def()
    }
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type CronJobRun = Model;
