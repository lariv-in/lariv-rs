use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::hr::gender::ApplicantGender;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
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
    #[sea_orm(indexed)]
    pub form_response_id: Option<i64>,
    /// Age stored as a duration in nanoseconds.
    pub age: Option<i64>,
    pub gender: Option<ApplicantGender>,
    #[sea_orm(indexed)]
    pub resume_vnode_id: Option<i64>,
    #[sea_orm(indexed)]
    pub job_form_id: Option<i64>,
    pub remarks: Option<String>,
    pub address: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::UserId",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "crate::plugins::forms::entities::form_response::Entity",
        from = "Column::FormResponseId",
        to = "crate::plugins::forms::entities::form_response::Column::Id",
        on_delete = "SetNull"
    )]
    FormResponse,
    #[sea_orm(
        belongs_to = "crate::plugins::filesystem::entities::filesystem_node::Entity",
        from = "Column::ResumeVnodeId",
        to = "crate::plugins::filesystem::entities::filesystem_node::Column::Id",
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

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<crate::plugins::forms::entities::form_response::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FormResponse.def()
    }
}

impl Related<crate::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
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
