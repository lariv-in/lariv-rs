use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::forms::types::FormQuestions;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "forms")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub title: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub questions: FormQuestions,
    pub created_by_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::CreatedById",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    CreatedBy,
    #[sea_orm(has_many = "super::form_response::Entity")]
    Responses,
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedBy.def()
    }
}

impl Related<super::form_response::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Responses.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type Form = Model;
