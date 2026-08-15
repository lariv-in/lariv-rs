//! `list_skills` / `get_skill_detail`.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    db::trigram,
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
                "List assistant skills (name and description). Pass query to fuzzy-search by name or description using trigram matching."
                    .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Optional trigram / substring search" },
                    "limit": { "type": "integer", "description": "Max results when query is set (default 20)" }
                },
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        #[derive(Debug, Deserialize, Default)]
        struct ListArgs {
            #[serde(default)]
            query: String,
            #[serde(default)]
            limit: u64,
        }
        let parsed: ListArgs = serde_json::from_value(args).unwrap_or_default();
        let query = parsed.query.trim();
        let mut select = SkillEntity::find();
        if !query.is_empty() {
            select = trigram::apply_text_search(
                select,
                ctx.db.get_database_backend(),
                &[skill::Column::Name, skill::Column::Description],
                query,
            )
            .limit(trigram::clamp_search_limit(parsed.limit));
        } else {
            select = select.order_by_asc(skill::Column::Name);
        }
        let skills = select.all(ctx.db).await.map_err(|e| e.to_string())?;
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
            description: "Retrieve a skill by name (exact, then trigram fuzzy match), including content and associated file paths.".into(),
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
        let skill = find_skill_by_name(ctx.db, name).await?;

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

async fn find_skill_by_name(
    db: &sea_orm::DatabaseConnection,
    name: &str,
) -> Result<skill::Model, String> {
    if let Some(skill) = SkillEntity::find()
        .filter(skill::Column::Name.eq(name))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    {
        return Ok(skill);
    }
    let matches = trigram::search::<SkillEntity, _>(
        db,
        &[skill::Column::Name, skill::Column::Description],
        name,
        8,
    )
    .await
    .map_err(|e| e.to_string())?;
    match matches.len() {
        0 => Err(format!("skill not found: {name}")),
        1 => Ok(matches.into_iter().next().expect("one skill")),
        _ => Err(format!(
            "multiple skills match {name:?}: {}",
            matches
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}
