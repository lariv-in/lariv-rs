//! Rune sandbox bindings for website route lookup and edits.
//!
//! Deleting a route is registered on the HITL capability instead, so a script
//! cannot drop a public URL without approval.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers website route helpers onto the assistant Rune environment.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl RuneEnvRegistrar for Hook {
    fn register_rune_env(self, rune_env: &mut RuneEnvCapability) {
        #[cfg(feature = "cap-llm")]
        register(rune_env);
        #[cfg(not(feature = "cap-llm"))]
        let _ = rune_env;
    }
}

#[cfg(feature = "cap-llm")]
fn register(rune_env: &mut RuneEnvCapability) {
    use std::sync::Arc;

    use lariv_core::rune_env::NativeBinding;

    rune_env.register_contextual(
        "create_website_route",
        "create_website_route(#{ path: string, page_id: int, is_active?: bool, theme?: string, reference_ids?: [int] }) -> int  // new route id. path is the public URL path as stored (whitespace trimmed; the public matcher ignores a leading or trailing slash). page_id is an existing file VNode — create the page with create_vnode first. is_active defaults to true. theme is a GrapesJS theme key, or empty for none. reference_ids are extra file VNode ids the page may read.",
        |_ctx| NativeBinding::Function(Arc::new(create_website_route)),
    );
    rune_env.register_contextual(
        "update_website_route",
        "update_website_route(#{ id: int, path?: string, page_id?: int, is_active?: bool, theme?: string, reference_ids?: [int] }) -> int  // updated route id. Omitted fields stay unchanged. reference_ids, when present, replaces the reference set (pass [] to clear). Page HTML stays on the VNode; use edit_vnode to change it. Does not touch the GrapesJS project JSON.",
        |_ctx| NativeBinding::Function(Arc::new(update_website_route)),
    );
    rune_env.register_contextual(
        "get_website_route",
        "get_website_route(#{ id: int }) -> #{ id: int, path: string, page_id: int, is_active: bool, theme: string, reference_ids: [int] }",
        |_ctx| NativeBinding::Function(Arc::new(get_website_route)),
    );
    rune_env.register_contextual(
        "search_website_routes",
        "search_website_routes(#{ query: string, limit?: int }) -> #{ results: [#{ id: int, path: string, page_id: int, is_active: bool, theme: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_website_routes)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_website_route(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let input = parse_create_args(args)?;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move { insert_route(&db, input).await })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn update_website_route(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let input = parse_update_args(args)?;
    let db = ctx.db.clone();
    let id = input.id;
    lariv_core::rune_env::block_on_async(async move { patch_route(&db, input).await })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn get_website_route(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id_arg(args, "get_website_route")?;
    let db = ctx.db.clone();
    let row = lariv_core::rune_env::block_on_async(async move { load_route(&db, id).await })?;
    lariv_core::rune_env::json_to_rune(row)
}

#[cfg(feature = "cap-llm")]
fn search_website_routes(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    use serde::Deserialize;
    use serde_json::json;

    use crate::entities::db_route::{self, Entity as DbRouteEntity};
    use lariv_core::db::trigram;
    use lariv_core::rune_env::{block_on_async, json_to_rune};

    #[derive(Debug, Deserialize, Default)]
    struct SearchArgs {
        #[serde(default)]
        query: String,
        #[serde(default)]
        limit: u64,
    }

    let parsed: SearchArgs = serde_json::from_value(object_arg(args, "search_website_routes")?)
        .map_err(|e| format!("invalid search_website_routes arguments: {e}"))?;
    let query = parsed.query.trim().to_string();
    if query.is_empty() {
        return Err("search_website_routes requires query".into());
    }
    let limit = trigram::clamp_search_limit(parsed.limit);
    let db = ctx.db.clone();
    let rows = block_on_async(async move {
        trigram::search::<DbRouteEntity, _>(&db, &[db_route::Column::Path], &query, limit).await
    })
    .map_err(|e| e.to_string())?;
    json_to_rune(json!({
        "results": rows.into_iter().map(|route| json!({
            "id": route.id,
            "path": route.path,
            "page_id": route.page_id,
            "is_active": route.is_active,
            "theme": route.theme,
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(feature = "cap-llm")]
fn object_arg(args: &[rune::Value], name: &str) -> Result<serde_json::Value, String> {
    let value = args
        .first()
        .ok_or_else(|| format!("{name} requires an object argument"))?;
    lariv_core::rune_env::rune_to_json(value).map_err(|e| format!("invalid {name} arguments: {e}"))
}

#[cfg(feature = "cap-llm")]
fn parse_id_arg(args: &[rune::Value], name: &str) -> Result<i64, String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct IdArgs {
        id: i64,
    }

    let parsed: IdArgs = serde_json::from_value(object_arg(args, name)?)
        .map_err(|e| format!("invalid {name} arguments: {e}"))?;
    if parsed.id <= 0 {
        return Err(format!("{name} requires a positive route id"));
    }
    Ok(parsed.id)
}

#[cfg(feature = "cap-llm")]
struct CreateInput {
    path: String,
    page_id: i64,
    is_active: bool,
    theme: String,
    reference_ids: Vec<i64>,
}

#[cfg(feature = "cap-llm")]
struct UpdateInput {
    id: i64,
    path: Option<String>,
    page_id: Option<i64>,
    is_active: Option<bool>,
    theme: Option<String>,
    reference_ids: Option<Vec<i64>>,
}

#[cfg(feature = "cap-llm")]
fn parse_create_args(args: &[rune::Value]) -> Result<CreateInput, String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct CreateArgs {
        path: String,
        page_id: i64,
        #[serde(default = "default_active")]
        is_active: bool,
        #[serde(default)]
        theme: String,
        #[serde(default)]
        reference_ids: Vec<i64>,
    }

    fn default_active() -> bool {
        true
    }

    let parsed: CreateArgs = serde_json::from_value(object_arg(args, "create_website_route")?)
        .map_err(|e| format!("invalid create_website_route arguments: {e}"))?;
    let path = normalize_path(&parsed.path)?;
    if parsed.page_id <= 0 {
        return Err("create_website_route requires a positive page_id".into());
    }
    let theme = normalize_theme(parsed.theme)?;
    let reference_ids = normalize_refs(&parsed.reference_ids)?;
    Ok(CreateInput {
        path,
        page_id: parsed.page_id,
        is_active: parsed.is_active,
        theme,
        reference_ids,
    })
}

#[cfg(feature = "cap-llm")]
fn parse_update_args(args: &[rune::Value]) -> Result<UpdateInput, String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct UpdateArgs {
        id: i64,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        page_id: Option<i64>,
        #[serde(default)]
        is_active: Option<bool>,
        #[serde(default)]
        theme: Option<String>,
        #[serde(default)]
        reference_ids: Option<Vec<i64>>,
    }

    let parsed: UpdateArgs = serde_json::from_value(object_arg(args, "update_website_route")?)
        .map_err(|e| format!("invalid update_website_route arguments: {e}"))?;
    if parsed.id <= 0 {
        return Err("update_website_route requires a positive route id".into());
    }
    let path = match parsed.path {
        Some(path) => Some(normalize_path(&path)?),
        None => None,
    };
    if let Some(page_id) = parsed.page_id
        && page_id <= 0
    {
        return Err("update_website_route requires a positive page_id".into());
    }
    let theme = match parsed.theme {
        Some(theme) => Some(normalize_theme(theme)?),
        None => None,
    };
    let reference_ids = match parsed.reference_ids {
        Some(ids) => Some(normalize_refs(&ids)?),
        None => None,
    };
    Ok(UpdateInput {
        id: parsed.id,
        path,
        page_id: parsed.page_id,
        is_active: parsed.is_active,
        theme,
        reference_ids,
    })
}

#[cfg(feature = "cap-llm")]
fn normalize_path(path: &str) -> Result<String, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("path is required".into());
    }
    Ok(path.to_string())
}

#[cfg(feature = "cap-llm")]
fn normalize_theme(theme: String) -> Result<String, String> {
    let theme = theme.trim().to_string();
    if theme.chars().count() > 128 {
        return Err("theme must be at most 128 characters".into());
    }
    Ok(theme)
}

#[cfg(feature = "cap-llm")]
fn normalize_refs(ids: &[i64]) -> Result<Vec<i64>, String> {
    let mut out = Vec::new();
    for id in ids {
        if *id <= 0 {
            return Err("reference_ids must be positive vnode ids".into());
        }
        if !out.contains(id) {
            out.push(*id);
        }
    }
    Ok(out)
}

#[cfg(feature = "cap-llm")]
async fn require_file_vnode(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    label: &str,
) -> Result<(), String> {
    use lariv_plugin_filesystem::node;

    let vnode = node::get_by_id(db, id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{label} {id} not found"))?;
    if vnode.is_directory {
        return Err(format!("{label} {id} is a directory"));
    }
    Ok(())
}

#[cfg(feature = "cap-llm")]
async fn ensure_path_free(
    db: &sea_orm::DatabaseConnection,
    path: &str,
    except_id: Option<i64>,
) -> Result<(), String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    use crate::entities::db_route::{self, Entity as DbRouteEntity};

    let mut query = DbRouteEntity::find().filter(db_route::Column::Path.eq(path));
    if let Some(id) = except_id {
        query = query.filter(db_route::Column::Id.ne(id));
    }
    if query.one(db).await.map_err(|e| e.to_string())?.is_some() {
        return Err(format!("a route already uses path {path}"));
    }
    Ok(())
}

#[cfg(feature = "cap-llm")]
async fn insert_route(db: &sea_orm::DatabaseConnection, input: CreateInput) -> Result<i64, String> {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};

    use crate::entities::db_route::ActiveModel;
    use crate::handlers::routes::sync_refs;

    require_file_vnode(db, input.page_id, "page").await?;
    for id in &input.reference_ids {
        require_file_vnode(db, *id, "reference").await?;
    }
    ensure_path_free(db, &input.path, None).await?;
    let now = Utc::now();
    let route = ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        path: Set(input.path),
        page_id: Set(input.page_id),
        is_active: Set(input.is_active),
        theme: Set(input.theme),
        grapes_project: Set(None),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
    if let Err(e) = sync_refs(db, route.id, &input.reference_ids).await {
        let rollback = crate::entities::DbRouteEntity::delete_by_id(route.id)
            .exec(db)
            .await;
        if let Err(del) = rollback {
            return Err(format!(
                "{e} (also failed to roll back route {}: {del})",
                route.id
            ));
        }
        return Err(e.to_string());
    }
    Ok(route.id)
}

#[cfg(feature = "cap-llm")]
async fn patch_route(db: &sea_orm::DatabaseConnection, input: UpdateInput) -> Result<(), String> {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};

    use crate::entities::db_route::{ActiveModel, Entity as DbRouteEntity};
    use crate::handlers::routes::sync_refs;

    let existing = DbRouteEntity::find_by_id(input.id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("website route {} not found", input.id))?;
    if let Some(path) = &input.path
        && path != &existing.path
    {
        ensure_path_free(db, path, Some(input.id)).await?;
    }
    if let Some(page_id) = input.page_id {
        require_file_vnode(db, page_id, "page").await?;
    }
    if let Some(ids) = &input.reference_ids {
        for id in ids {
            require_file_vnode(db, *id, "reference").await?;
        }
    }
    let mut am: ActiveModel = existing.into();
    let mut changed = false;
    if let Some(path) = input.path {
        am.path = Set(path);
        changed = true;
    }
    if let Some(page_id) = input.page_id {
        am.page_id = Set(page_id);
        changed = true;
    }
    if let Some(is_active) = input.is_active {
        am.is_active = Set(is_active);
        changed = true;
    }
    if let Some(theme) = input.theme {
        am.theme = Set(theme);
        changed = true;
    }
    if changed {
        am.updated_at = Set(Some(Utc::now()));
        am.update(db).await.map_err(|e| e.to_string())?;
    }
    if let Some(ids) = input.reference_ids {
        sync_refs(db, input.id, &ids)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(feature = "cap-llm")]
async fn load_route(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Result<serde_json::Value, String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use serde_json::json;

    use crate::entities::db_route::Entity as DbRouteEntity;
    use crate::entities::route_reference::{self, Entity as RouteRefEntity};

    let route = DbRouteEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("website route {id} not found"))?;
    let links = RouteRefEntity::find()
        .filter(route_reference::Column::DbRouteId.eq(id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let reference_ids: Vec<i64> = links.into_iter().map(|link| link.v_node_id).collect();
    Ok(json!({
        "id": route.id,
        "path": route.path,
        "page_id": route.page_id,
        "is_active": route.is_active,
        "theme": route.theme,
        "reference_ids": reference_ids,
    }))
}

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx};
    use lariv_plugin_filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use lariv_plugin_llm_assistant::rune_engine;

    fn test_env_ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: &'a Arc<DynFilestore>,
    ) -> RuneEnvCtx<'a> {
        RuneEnvCtx {
            db,
            store: lariv_plugin_filesystem::storage::erase_store(Arc::clone(store)),
            session_id: None,
        }
    }

    fn registered_env() -> RuneEnvCapability {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        cap
    }

    fn error_of(out: &serde_json::Value) -> String {
        out.get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn registers_route_crud() {
        let names = registered_env().all_names();
        for name in [
            "create_website_route",
            "update_website_route",
            "get_website_route",
            "search_website_routes",
        ] {
            assert!(
                names.iter().any(|registered| registered == name),
                "expected {name} in {names:?}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, "create_website_route(())", &[]).await;
        let error = error_of(&out);
        assert!(
            error.contains("create_website_route requires an object argument")
                || error.contains("unsupported")
                || error.contains("invalid create_website_route arguments"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_rejects_empty_path() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_website_route(#{ path: "  ", page_id: 1 })"#,
            &[],
        )
        .await;
        let error = error_of(&out);
        assert!(
            error.contains("path is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_rejects_non_positive_page_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_website_route(#{ path: "/about", page_id: 0 })"#,
            &[],
        )
        .await;
        let error = error_of(&out);
        assert!(
            error.contains("positive page_id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_rejects_bad_reference_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_website_route(#{ path: "/about", page_id: 1, reference_ids: [0] })"#,
            &[],
        )
        .await;
        let error = error_of(&out);
        assert!(
            error.contains("reference_ids"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_rejects_missing_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_website_route(#{ path: "/about" })"#,
            &[],
        )
        .await;
        let error = error_of(&out);
        assert!(
            error.contains("invalid update_website_route arguments") || error.contains("id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_website_route(#{ id: 0, path: "/about" })"#,
            &[],
        )
        .await;
        let error = error_of(&out);
        assert!(
            error.contains("positive route id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, r#"get_website_route(#{ id: 0 })"#, &[])
                .await;
        let error = error_of(&out);
        assert!(
            error.contains("positive route id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_rejects_empty_query() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"search_website_routes(#{ query: "  " })"#,
            &[],
        )
        .await;
        let error = error_of(&out);
        assert!(
            error.contains("requires query"),
            "unexpected error payload: {out}"
        );
    }
}
