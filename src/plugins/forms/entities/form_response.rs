use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::forms::types::FormAnswers;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "form_responses")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub form_id: i64,
    #[sea_orm(column_type = "JsonBinary")]
    pub answers: FormAnswers,
    pub submitted_at: DateTime<Utc>,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::form::Entity",
        from = "Column::FormId",
        to = "super::form::Column::Id",
        on_delete = "Cascade"
    )]
    Form,
}

impl Related<super::form::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Form.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type FormResponse = Model;
