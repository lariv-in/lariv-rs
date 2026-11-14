use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "test_cases")]
/// One stored assistant test case.
///
/// `definition` is markdown instructions given to the assistant.
/// `solution` is a Rune script whose `main` returns `Result<Result>`:
/// an inner `Ok` passes the case, and an inner `Err` fails it with that error.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub definition: String,
    #[sea_orm(column_type = "Text")]
    pub solution: String,
    pub created_by_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::CreatedById",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    CreatedBy,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedBy.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type TestCase = Model;
