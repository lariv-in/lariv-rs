use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Join table (`Skill` ↔ `VNode`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_skill_files")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub skill_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub v_node_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::skill::Entity",
        from = "Column::SkillId",
        to = "super::skill::Column::Id",
        on_delete = "Cascade"
    )]
    Skill,
    #[sea_orm(
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::VNodeId",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Cascade"
    )]
    VNode,
}

impl Related<super::skill::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Skill.def()
    }
}

impl Related<lariv_plugin_filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VNode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type SkillFileLink = Model;
