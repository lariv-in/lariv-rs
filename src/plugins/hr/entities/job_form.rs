use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_job_forms")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub job_title: String,
    pub salary_range: Option<String>,
    pub experience_required: Option<String>,
    pub description: String,
    #[sea_orm(indexed)]
    pub form_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::forms::entities::form::Entity",
        from = "Column::FormId",
        to = "crate::plugins::forms::entities::form::Column::Id",
        on_delete = "Restrict"
    )]
    Form,
}

impl Related<crate::plugins::forms::entities::form::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Form.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
