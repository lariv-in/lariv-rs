use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "passports")]
pub struct Model {
    /// Integer primary key. `documents.document_type_id` stores this value when
    /// `document_type` is `passport`.
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub vnode_id: i64,
    #[sea_orm(unique)]
    pub passport_number: String,
    pub name: String,
    pub gender: crate::plugins::documents::gender::Gender,
    pub date_of_birth: NaiveDate,
    pub nationality: String,
    pub expiry_date: NaiveDate,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::filesystem::entities::filesystem_node::Entity",
        from = "Column::VnodeId",
        to = "crate::plugins::filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Restrict"
    )]
    VNode,
}

impl Related<crate::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VNode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
