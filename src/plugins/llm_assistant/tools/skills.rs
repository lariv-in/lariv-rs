//! `list_skills` / `get_skill_detail`.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::{
        filesystem::node,
        llm_assistant::{
            entities::{
                skill::{self, Entity as SkillEntity},
                skill_file_link,
            },
            genai::FunctionDeclaration,
        },
    },
};

pub struct ListSkillsTool;

#[async_trait]
impl LlmTool for ListSkillsTool {
    fn name(&self) -> &str {
        "list_skills"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "list_skills".into(),
            description:
                "Retrieve a list of all assistant skills, including their names and descriptions."
                    .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {},
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, _args: Value) -> Result<Value, String> {
        let skills = SkillEntity::find()
            .filter(skill::Column::DeletedAt.is_null())
            .order_by_asc(skill::Column::Name)
            .all(ctx.db)
            .await
            .map_err(|e| e.to_string())?;
        let out: Vec<Value> = skills
            .into_iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "description": s.description,
                })
            })
            .collect();
        Ok(json!({ "skills": out }))
    }
}

pub struct GetSkillDetailTool;

#[derive(Debug, Deserialize, Default)]
struct DetailArgs {
    #[serde(default)]
    name: String,
}

#[async_trait]
impl LlmTool for GetSkillDetailTool {
    fn name(&self) -> &str {
        "get_skill_detail"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "get_skill_detail".into(),
            description: "Retrieve detailed description of a skill by name, including its content and associated file paths.".into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Skill name" }
                },
                "required": ["name"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: DetailArgs = serde_json::from_value(args).unwrap_or_default();
        let name = parsed.name.trim();
        if name.is_empty() {
            return Err("skill name is required".into());
        }
        let skill = SkillEntity::find()
            .filter(skill::Column::DeletedAt.is_null())
            .filter(skill::Column::Name.eq(name))
            .one(ctx.db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("skill not found: {name}"))?;

        let links = skill_file_link::Entity::find()
            .filter(skill_file_link::Column::SkillId.eq(skill.id))
            .all(ctx.db)
            .await
            .map_err(|e| e.to_string())?;

        let mut file_paths = Vec::new();
        for link in links {
            if let Some(vnode) = node::get_by_id(ctx.db, link.v_node_id)
                .await
                .map_err(|e| e.to_string())?
            {
                file_paths.push(node::get_path(ctx.db, &vnode).await);
            }
        }

        Ok(json!({
            "name": skill.name,
            "description": skill.description,
            "content": skill.content,
            "file_paths": file_paths,
        }))
    }
}
