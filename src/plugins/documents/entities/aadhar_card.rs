use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::documents::gender::Gender;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "aadhar_cards")]
pub struct Model {
    /// Integer primary key. `documents.document_type_id` stores this value when
    /// `document_type` is `aadhar_card`.
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// Uploaded Aadhaar card file.
    pub vnode_id: i64,
    #[sea_orm(unique)]
    pub aadhar_number: String,
    pub name: String,
    pub gender: Gender,
    pub date_of_birth: NaiveDate,
    pub address: String,
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
