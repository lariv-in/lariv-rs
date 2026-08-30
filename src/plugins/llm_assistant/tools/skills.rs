//! `list_skills` / `get_skill_detail` / `create_skill` / `edit_skill`.

use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
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
            handlers::skills::sync_skill_files,
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
        skill_detail_json(ctx.db, skill).await
    }
}

pub struct CreateSkillTool;

#[derive(Debug, Deserialize, Default)]
struct CreateArgs {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    file_paths: Vec<String>,
}

#[async_trait]
impl LlmTool for CreateSkillTool {
    fn name(&self) -> &str {
        "create_skill"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "create_skill".into(),
            description: "Create a new assistant skill with name, description, content (instructions), and optional associated file paths (absolute VNode paths).".into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Unique skill name" },
                    "description": { "type": "string", "description": "Short description for discovery via list_skills" },
                    "content": { "type": "string", "description": "Skill instructions the assistant should follow" },
                    "file_paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Absolute VNode paths to associate with the skill"
                    }
                },
                "required": ["name", "content"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: CreateArgs = serde_json::from_value(args).unwrap_or_default();
        let name = parsed.name.trim();
        if name.is_empty() {
            return Err("skill name is required".into());
        }
        let content = parsed.content.trim();
        if content.is_empty() {
            return Err("content is required".into());
        }

        let existing = SkillEntity::find()
            .filter(skill::Column::Name.eq(name))
            .one(ctx.db)
            .await
            .map_err(|e| e.to_string())?;
        if existing.is_some() {
            return Err(format!("skill name already exists: {name}"));
        }

        let file_ids = resolve_file_paths(ctx.db, &parsed.file_paths).await?;

        let now = Utc::now();
        let saved = skill::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            name: Set(name.to_string()),
            description: Set(parsed.description),
            content: Set(content.to_string()),
        }
        .insert(ctx.db)
        .await
        .map_err(|e| e.to_string())?;

        if !file_ids.is_empty() {
            sync_skill_files(ctx.db, saved.id, &file_ids)
                .await
                .map_err(|e| e.to_string())?;
        }

        skill_detail_json(ctx.db, saved).await
    }
}

pub struct EditSkillTool;

#[derive(Debug, Deserialize, Default)]
struct EditArgs {
    #[serde(default)]
    name: String,
    new_name: Option<String>,
    description: Option<String>,
    content: Option<String>,
    file_paths: Option<Vec<String>>,
}

#[async_trait]
impl LlmTool for EditSkillTool {
    fn name(&self) -> &str {
        "edit_skill"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "edit_skill".into(),
            description: "Update an existing assistant skill. Identify it by name (exact, then trigram fuzzy match). Pass only the fields that should change. file_paths replaces associated files (absolute VNode paths); omit it to leave files unchanged.".into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Current skill name" },
                    "new_name": { "type": "string", "description": "Rename the skill" },
                    "description": { "type": "string", "description": "Replacement description" },
                    "content": { "type": "string", "description": "Replacement skill instructions" },
                    "file_paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Absolute VNode paths to associate with the skill (replaces existing files)"
                    }
                },
                "required": ["name"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: EditArgs = serde_json::from_value(args).unwrap_or_default();
        let name = parsed.name.trim();
        if name.is_empty() {
            return Err("skill name is required".into());
        }

        let new_name = parsed
            .new_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if parsed
            .new_name
            .as_deref()
            .is_some_and(|s| s.trim().is_empty())
        {
            return Err("new_name must not be empty".into());
        }

        let content = match parsed.content.as_deref() {
            Some(c) if c.trim().is_empty() => {
                return Err("content must not be empty".into());
            }
            Some(c) => Some(c.to_string()),
            None => None,
        };

        if new_name.is_none()
            && parsed.description.is_none()
            && content.is_none()
            && parsed.file_paths.is_none()
        {
            return Err(
                "at least one of new_name, description, content, or file_paths is required".into(),
            );
        }

        let skill = find_skill_by_name(ctx.db, name).await?;
        let skill_id = skill.id;

        if let Some(ref renamed) = new_name
            && renamed != &skill.name
        {
            let clash = SkillEntity::find()
                .filter(skill::Column::Name.eq(renamed.clone()))
                .one(ctx.db)
                .await
                .map_err(|e| e.to_string())?;
            if clash.is_some_and(|other| other.id != skill_id) {
                return Err(format!("skill name already exists: {renamed}"));
            }
        }

        let file_ids = if let Some(paths) = parsed.file_paths.as_ref() {
            Some(resolve_file_paths(ctx.db, paths).await?)
        } else {
            None
        };

        let mut am: skill::ActiveModel = skill.into();
        if let Some(renamed) = new_name {
            am.name = Set(renamed);
        }
        if let Some(description) = parsed.description {
            am.description = Set(description);
        }
        if let Some(content) = content {
            am.content = Set(content);
        }
        am.updated_at = Set(Some(Utc::now()));
        let updated = am.update(ctx.db).await.map_err(|e| e.to_string())?;

        if let Some(file_ids) = file_ids {
            sync_skill_files(ctx.db, skill_id, &file_ids)
                .await
                .map_err(|e| e.to_string())?;
        }

        skill_detail_json(ctx.db, updated).await
    }
}

async fn skill_detail_json(
    db: &sea_orm::DatabaseConnection,
    skill: skill::Model,
) -> Result<Value, String> {
    let links = skill_file_link::Entity::find()
        .filter(skill_file_link::Column::SkillId.eq(skill.id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    let mut file_paths = Vec::new();
    for link in links {
        if let Some(vnode) = node::get_by_id(db, link.v_node_id)
            .await
            .map_err(|e| e.to_string())?
        {
            file_paths.push(node::get_path(db, &vnode).await);
        }
    }

    Ok(json!({
        "name": skill.name,
        "description": skill.description,
        "content": skill.content,
        "file_paths": file_paths,
    }))
}

async fn resolve_file_paths(
    db: &sea_orm::DatabaseConnection,
    paths: &[String],
) -> Result<Vec<i64>, String> {
    let mut ids = Vec::new();
    for raw in paths {
        let path = raw.trim();
        if path.is_empty() {
            return Err("file path must not be empty".into());
        }
        let (node, _) = node::get_by_path(db, path)
            .await
            .map_err(|e| e.to_string())?;
        let Some(vnode) = node else {
            return Err(format!("file not found at path \"{path}\""));
        };
        if vnode.is_directory {
            return Err(format!("path \"{path}\" is a directory, not a file"));
        }
        if !ids.contains(&vnode.id) {
            ids.push(vnode.id);
        }
    }
    Ok(ids)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, Schema};

    use crate::{
        llm_tools::ToolCtx,
        plugins::{
            filesystem::{
                entities::filesystem_node,
                node::{self, NodeFile},
                storage::{DynFilestore, LocalFilestore, UnimplementedFilestore},
            },
            llm_assistant::entities::skill_file_link,
        },
        rune_env::RuneEnvCapability,
    };

    async fn setup_db() -> sea_orm::DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        db.execute(backend.build(&schema.create_table_from_entity(skill::Entity)))
            .await
            .expect("skills");
        db.execute(backend.build(&schema.create_table_from_entity(filesystem_node::Entity)))
            .await
            .expect("vnodes");
        db.execute(backend.build(&schema.create_table_from_entity(skill_file_link::Entity)))
            .await
            .expect("links");
        db
    }

    async fn insert_skill(
        db: &sea_orm::DatabaseConnection,
        name: &str,
        description: &str,
        content: &str,
    ) -> skill::Model {
        let now = Utc::now();
        skill::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            name: Set(name.into()),
            description: Set(description.into()),
            content: Set(content.into()),
        }
        .insert(db)
        .await
        .expect("skill")
    }

    fn unimplemented_ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        rune_env: &'a RuneEnvCapability,
        store: Arc<DynFilestore>,
    ) -> ToolCtx<'a> {
        ToolCtx {
            db,
            store,
            cse_api_key: "",
            cse_cx: "",
            rune_env,
            session_id: None,
        }
    }

    #[tokio::test]
    async fn create_inserts_skill_with_optional_files() {
        let db = setup_db().await;
        let dir = std::env::temp_dir().join(format!("lariv-create-skill-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let store = LocalFilestore::new(dir.to_string_lossy());
        let store_ref: &DynFilestore = &store;

        node::create(
            &db,
            store_ref,
            "ref.rs".into(),
            false,
            Some(NodeFile::Bytes {
                filename: "ref.rs".into(),
                data: b"ref".to_vec(),
            }),
            None,
        )
        .await
        .expect("file");

        let rune_env = RuneEnvCapability::new();
        let store_arc: Arc<DynFilestore> = Arc::new(store);
        let ctx = unimplemented_ctx(&db, &rune_env, Arc::clone(&store_arc));

        let out = CreateSkillTool
            .run(
                &ctx,
                json!({
                    "name": "new-skill",
                    "description": "does a thing",
                    "content": "step one",
                    "file_paths": ["/ref.rs"],
                }),
            )
            .await
            .expect("create");

        assert_eq!(out["name"], "new-skill");
        assert_eq!(out["description"], "does a thing");
        assert_eq!(out["content"], "step one");
        assert_eq!(out["file_paths"], json!(["/ref.rs"]));
    }

    #[tokio::test]
    async fn create_rejects_empty_fields_and_duplicate_name() {
        let db = setup_db().await;
        let rune_env = RuneEnvCapability::new();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = unimplemented_ctx(&db, &rune_env, store);

        let err = CreateSkillTool
            .run(&ctx, json!({ "name": "", "content": "x" }))
            .await
            .unwrap_err();
        assert!(err.contains("name is required"));

        let err = CreateSkillTool
            .run(&ctx, json!({ "name": "x", "content": "  " }))
            .await
            .unwrap_err();
        assert!(err.contains("content is required"));

        CreateSkillTool
            .run(
                &ctx,
                json!({
                    "name": "dup",
                    "content": "c",
                }),
            )
            .await
            .expect("first");
        let err = CreateSkillTool
            .run(
                &ctx,
                json!({
                    "name": "dup",
                    "content": "other",
                }),
            )
            .await
            .unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[tokio::test]
    async fn edit_updates_content_and_description() {
        let db = setup_db().await;
        insert_skill(&db, "invoice", "old desc", "old content").await;
        let rune_env = RuneEnvCapability::new();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = unimplemented_ctx(&db, &rune_env, store);

        let out = EditSkillTool
            .run(
                &ctx,
                json!({
                    "name": "invoice",
                    "description": "new desc",
                    "content": "new content",
                }),
            )
            .await
            .expect("edit");

        assert_eq!(out["name"], "invoice");
        assert_eq!(out["description"], "new desc");
        assert_eq!(out["content"], "new content");

        let stored = SkillEntity::find()
            .one(&db)
            .await
            .expect("load")
            .expect("row");
        assert_eq!(stored.description, "new desc");
        assert_eq!(stored.content, "new content");
    }

    #[tokio::test]
    async fn edit_renames_skill() {
        let db = setup_db().await;
        insert_skill(&db, "old-name", "d", "c").await;
        let rune_env = RuneEnvCapability::new();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = unimplemented_ctx(&db, &rune_env, store);

        let out = EditSkillTool
            .run(
                &ctx,
                json!({
                    "name": "old-name",
                    "new_name": "new-name",
                }),
            )
            .await
            .expect("rename");
        assert_eq!(out["name"], "new-name");
        assert_eq!(out["content"], "c");
    }

    #[tokio::test]
    async fn edit_rejects_missing_skill_and_empty_patch() {
        let db = setup_db().await;
        let rune_env = RuneEnvCapability::new();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = unimplemented_ctx(&db, &rune_env, store);

        let err = EditSkillTool
            .run(&ctx, json!({ "name": "missing", "content": "x" }))
            .await
            .unwrap_err();
        assert!(err.contains("skill not found"));

        insert_skill(&db, "keep", "d", "c").await;
        let err = EditSkillTool
            .run(&ctx, json!({ "name": "keep" }))
            .await
            .unwrap_err();
        assert!(err.contains("at least one of"));
    }

    #[tokio::test]
    async fn edit_rejects_duplicate_name() {
        let db = setup_db().await;
        insert_skill(&db, "alpha", "d", "c").await;
        insert_skill(&db, "beta", "d", "c").await;
        let rune_env = RuneEnvCapability::new();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = unimplemented_ctx(&db, &rune_env, store);

        let err = EditSkillTool
            .run(
                &ctx,
                json!({
                    "name": "alpha",
                    "new_name": "beta",
                }),
            )
            .await
            .unwrap_err();
        assert!(err.contains("already exists"));
    }

    #[tokio::test]
    async fn edit_replaces_file_paths_and_leaves_them_when_omitted() {
        let db = setup_db().await;
        let dir = std::env::temp_dir().join(format!("lariv-edit-skill-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let store = LocalFilestore::new(dir.to_string_lossy());
        let store_ref: &DynFilestore = &store;

        let a = node::create(
            &db,
            store_ref,
            "a.rs".into(),
            false,
            Some(NodeFile::Bytes {
                filename: "a.rs".into(),
                data: b"a".to_vec(),
            }),
            None,
        )
        .await
        .expect("a");
        node::create(
            &db,
            store_ref,
            "b.rs".into(),
            false,
            Some(NodeFile::Bytes {
                filename: "b.rs".into(),
                data: b"b".to_vec(),
            }),
            None,
        )
        .await
        .expect("b");

        let skill = insert_skill(&db, "coded", "d", "c").await;
        sync_skill_files(&db, skill.id, &[a.id])
            .await
            .expect("link a");

        let rune_env = RuneEnvCapability::new();
        let store_arc: Arc<DynFilestore> = Arc::new(store);
        let ctx = unimplemented_ctx(&db, &rune_env, Arc::clone(&store_arc));

        let out = EditSkillTool
            .run(
                &ctx,
                json!({
                    "name": "coded",
                    "file_paths": ["/b.rs"],
                }),
            )
            .await
            .expect("relink");
        assert_eq!(out["file_paths"], json!(["/b.rs"]));

        let out = EditSkillTool
            .run(
                &ctx,
                json!({
                    "name": "coded",
                    "description": "still linked",
                }),
            )
            .await
            .expect("omit files");
        assert_eq!(out["description"], "still linked");
        assert_eq!(out["file_paths"], json!(["/b.rs"]));
    }
}
