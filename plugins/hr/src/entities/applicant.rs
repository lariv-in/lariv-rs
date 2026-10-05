use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::gender::ApplicantGender;
use crate::questions::JobPostingAnswers;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_applicants")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(indexed)]
    pub user_id: i64,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub date_of_birth: Option<DateTime<Utc>>,
    pub gender: Option<ApplicantGender>,
    #[sea_orm(indexed)]
    pub resume_vnode_id: Option<i64>,
    #[sea_orm(indexed)]
    pub job_form_id: Option<i64>,
    pub remarks: Option<String>,
    pub address: Option<String>,
    pub answers: JobPostingAnswers,
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
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::ResumeVnodeId",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_delete = "SetNull"
    )]
    Resume,
    #[sea_orm(
        belongs_to = "super::job_form::Entity",
        from = "Column::JobFormId",
        to = "super::job_form::Column::Id",
        on_delete = "SetNull"
    )]
    JobForm,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<lariv_plugin_filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Resume.def()
    }
}

impl Related<super::job_form::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JobForm.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
