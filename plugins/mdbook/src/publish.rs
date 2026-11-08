//! Export a book from the virtual filesystem, build it, and publish website routes.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use tokio::io::AsyncReadExt;

use lariv_plugin_filesystem::{
    entities::VNode,
    node::{self, NodeFile},
    storage::DynFilestore,
};
use lariv_plugin_website::{
    entities::db_route::{self, Entity as DbRouteEntity},
    render::RAW_FILE_THEME,
};

use super::entities::MdBookPreferences;
use super::paths::{
    book_file_url, entry_redirect_html, normalize_prefix, route_under_prefix, site_url,
};

const ENTRY_FILE_NAME: &str = "__mdbook_entry.html";

/// Result of a successful [`publish`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    /// The source vnode is missing.
    MissingSource,
    /// The source directory has no `SUMMARY.md` yet.
    MissingSummary { source: String, relative: String },
    /// Built files were written and website routes were replaced.
    Published { files: usize, prefix: String },
}

/// Build the book described by preferences and replace its website routes.
pub async fn publish(
    db: &DatabaseConnection,
    store: &DynFilestore,
    prefs: &MdBookPreferences,
) -> anyhow::Result<PublishOutcome> {
    let prefix = normalize_prefix(&prefs.url_prefix).map_err(|e| anyhow::anyhow!(e))?;
    let Some(source_node) = node::get_by_id(db, prefs.source_vnode_id).await? else {
        return Ok(PublishOutcome::MissingSource);
    };
    if !source_node.is_directory {
        anyhow::bail!(
            "mdbook source directory {} is not a directory",
            prefs.source_vnode_id
        );
    }
    let Some(output_node) = node::get_by_id(db, prefs.output_vnode_id).await? else {
        anyhow::bail!(
            "mdbook output directory {} was not found",
            prefs.output_vnode_id
        );
    };
    if !output_node.is_directory {
        anyhow::bail!(
            "mdbook output directory {} is not a directory",
            prefs.output_vnode_id
        );
    }
    if directories_overlap(db, &source_node, &output_node).await? {
        anyhow::bail!("mdbook source and output directories must be different");
    }
    let source_path = node::get_path(db, &source_node).await;
    if let Some(relative) = missing_summary(db, store, &source_node).await? {
        return Ok(PublishOutcome::MissingSummary {
            source: source_path,
            relative,
        });
    }

    let temp = tempfile::tempdir().map_err(|e| anyhow::anyhow!("mdbook temp dir: {e}"))?;
    export_tree(db, store, &source_node, temp.path()).await?;

    let root = temp.path().to_path_buf();
    let site = site_url(&prefix);
    let built = tokio::task::spawn_blocking(move || {
        let build_dir = build_book(&root, &site)?;
        read_build_files(&build_dir)
    })
    .await
    .map_err(|e| anyhow::anyhow!("mdbook build task failed: {e}"))??;
    if !built.iter().any(|(rel, _)| rel == "index.html") {
        anyhow::bail!("mdbook build did not produce index.html");
    }

    clear_previous(db, store, &output_node, &prefix).await?;
    let output_id = output_node.id;
    let mut published = Vec::with_capacity(built.len());
    for (relative, bytes) in built {
        let vnode = write_built_file(db, store, output_id, &relative, bytes).await?;
        published.push((relative, vnode.id));
    }

    for (relative, page_id) in &published {
        insert_route(db, &book_file_url(&prefix, relative), *page_id).await?;
    }

    let index_url = book_file_url(&prefix, "index.html");
    if index_url != prefix {
        let entry = write_built_file(
            db,
            store,
            output_id,
            ENTRY_FILE_NAME,
            entry_redirect_html(&index_url).into_bytes(),
        )
        .await?;
        insert_route(db, &prefix, entry.id).await?;
    }

    tracing::info!(
        files = published.len(),
        %prefix,
        source_vnode_id = source_node.id,
        output_vnode_id = output_node.id,
        "mdbook: published site"
    );
    Ok(PublishOutcome::Published {
        files: published.len(),
        prefix,
    })
}

/// `book.src/SUMMARY.md` when that file is not in the source directory.
async fn missing_summary(
    db: &DatabaseConnection,
    store: &DynFilestore,
    source: &VNode,
) -> anyhow::Result<Option<String>> {
    let children = node::list_children(db, Some(source.id), false, "").await?;
    let src = if let Some(config) = children
        .iter()
        .find(|child| !child.is_directory && child.name == "book.toml")
    {
        let text = String::from_utf8(read_vnode(store, config).await?)
            .map_err(|_| anyhow::anyhow!("mdbook book.toml is not utf-8"))?;
        let parsed = mdbook_driver::config::Config::from_str(&text)
            .map_err(|error| anyhow::anyhow!("mdbook book.toml: {error}"))?;
        if !parsed.book.src.is_relative()
            || parsed
                .book
                .src
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            anyhow::bail!("mdbook book.src must stay inside the source directory");
        }
        parsed.book.src
    } else {
        PathBuf::from("src")
    };
    let relative = src.join("SUMMARY.md");
    let parts: Vec<_> = relative.components().collect();
    let mut parent_id = source.id;
    for (index, part) in parts.iter().enumerate() {
        let std::path::Component::Normal(name) = part else {
            anyhow::bail!("mdbook book.src must stay inside the source directory");
        };
        let name = name.to_string_lossy();
        let children = node::list_children(db, Some(parent_id), false, "").await?;
        let Some(next) = children
            .into_iter()
            .find(|child| child.name == name.as_ref())
        else {
            return Ok(Some(relative.display().to_string()));
        };
        let last = index + 1 == parts.len();
        if next.is_directory {
            if last {
                return Ok(Some(relative.display().to_string()));
            }
            parent_id = next.id;
            continue;
        }
        if last {
            return Ok(None);
        }
        return Ok(Some(relative.display().to_string()));
    }
    Ok(Some(relative.display().to_string()))
}

fn build_book(root: &Path, site: &str) -> anyhow::Result<PathBuf> {
    let mut book = mdbook_driver::MDBook::load(root)
        .map_err(|e| anyhow::anyhow!("mdbook load failed: {e}"))?;
    if let Err(error) = book.config.set("output.html.site-url", site) {
        tracing::warn!(%error, "mdbook: could not set output.html.site-url");
    }
    if book.config.build.build_dir.is_absolute() {
        anyhow::bail!("mdbook build.build-dir must be a relative path");
    }
    book.build()
        .map_err(|e| anyhow::anyhow!("mdbook build failed: {e}"))?;
    let dir = root.join(&book.config.build.build_dir);
    if !dir.starts_with(root) {
        anyhow::bail!("mdbook build directory escaped the book root");
    }
    if !dir.is_dir() {
        anyhow::bail!(
            "mdbook build directory {} is missing",
            book.config.build.build_dir.display()
        );
    }
    Ok(dir)
}

fn read_build_files(build_dir: &Path) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
    let mut paths = Vec::new();
    walk_files(build_dir, &mut paths)?;
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = path.strip_prefix(build_dir).map_err(|error| {
            anyhow::anyhow!("built file is outside the build directory: {error}")
        })?;
        let relative = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        if relative.is_empty() || relative.split('/').any(|p| p.is_empty() || p == "..") {
            anyhow::bail!("refusing built path \"{relative}\"");
        }
        let bytes =
            std::fs::read(&path).map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
        files.push((relative, bytes));
    }
    Ok(files)
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| anyhow::anyhow!("read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| anyhow::anyhow!("read dir entry: {e}"))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| anyhow::anyhow!("file type {}: {e}", path.display()))?;
        if kind.is_dir() {
            walk_files(&path, out)?;
        } else if kind.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

async fn clear_previous(
    db: &DatabaseConnection,
    store: &DynFilestore,
    output: &VNode,
    prefix: &str,
) -> anyhow::Result<()> {
    let mut page_ids = Vec::new();
    let children = node::list_children(db, Some(output.id), false, "").await?;
    for child in &children {
        collect_ids(db, child, &mut page_ids).await?;
    }
    let routes = DbRouteEntity::find().all(db).await?;
    for route in routes {
        let owned = route.theme == RAW_FILE_THEME && route_under_prefix(prefix, &route.path);
        if owned || page_ids.contains(&route.page_id) {
            DbRouteEntity::delete_by_id(route.id).exec(db).await?;
        }
    }
    node::delete_direct_children(db, store, Some(output.id)).await?;
    Ok(())
}

/// True when the directories are the same node or one is nested inside the other.
pub(crate) async fn directories_overlap(
    db: &DatabaseConnection,
    left: &VNode,
    right: &VNode,
) -> anyhow::Result<bool> {
    if left.id == right.id {
        return Ok(true);
    }
    Ok(contains_node(db, left.id, right).await? || contains_node(db, right.id, left).await?)
}

async fn contains_node(
    db: &DatabaseConnection,
    ancestor_id: i64,
    start: &VNode,
) -> anyhow::Result<bool> {
    let mut parent_id = start.parent_id;
    while let Some(pid) = parent_id {
        if pid == ancestor_id {
            return Ok(true);
        }
        parent_id = node::get_by_id(db, pid)
            .await?
            .and_then(|parent| parent.parent_id);
    }
    Ok(false)
}

async fn collect_ids(
    db: &DatabaseConnection,
    node: &VNode,
    out: &mut Vec<i64>,
) -> anyhow::Result<()> {
    out.push(node.id);
    if !node.is_directory {
        return Ok(());
    }
    let children = node::list_children(db, Some(node.id), false, "").await?;
    for child in children {
        Box::pin(collect_ids(db, &child, out)).await?;
    }
    Ok(())
}

async fn write_built_file(
    db: &DatabaseConnection,
    store: &DynFilestore,
    output_root: i64,
    relative: &str,
    bytes: Vec<u8>,
) -> anyhow::Result<VNode> {
    let mut parts: Vec<&str> = relative.split('/').filter(|p| !p.is_empty()).collect();
    let Some(file_name) = parts.pop() else {
        anyhow::bail!("built file path is empty");
    };
    let mut parent_id = output_root;
    if !parts.is_empty() {
        let segments: Vec<String> = parts.iter().map(|s| (*s).to_string()).collect();
        parent_id = node::ensure_directory_path(db, store, Some(output_root), &segments)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mdbook could not create {relative}"))?;
    }
    let parent = node::get_by_id(db, parent_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("mdbook output directory disappeared"))?;
    let name = file_name.to_string();
    node::create(
        db,
        store,
        name.clone(),
        false,
        Some(NodeFile::Bytes {
            filename: name,
            data: bytes,
        }),
        Some(&parent),
        None,
        None,
    )
    .await
    .map_err(|e| anyhow::anyhow!("mdbook store {relative}: {e}"))
}

async fn insert_route(db: &DatabaseConnection, path: &str, page_id: i64) -> anyhow::Result<()> {
    if let Some(existing) = DbRouteEntity::find()
        .filter(db_route::Column::Path.eq(path))
        .one(db)
        .await?
    {
        anyhow::bail!(
            "website route \"{path}\" already exists (id {}); remove it or choose another urlPrefix",
            existing.id
        );
    }
    let now = chrono::Utc::now();
    let route = db_route::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        path: Set(path.to_string()),
        page_id: Set(page_id),
        is_active: Set(true),
        theme: Set(RAW_FILE_THEME.to_string()),
        grapes_project: Set(None),
    };
    route.insert(db).await?;
    Ok(())
}

async fn export_tree(
    db: &DatabaseConnection,
    store: &DynFilestore,
    node: &VNode,
    dest: &Path,
) -> anyhow::Result<()> {
    if node.is_directory {
        tokio::fs::create_dir_all(dest)
            .await
            .map_err(|e| anyhow::anyhow!("create {}: {e}", dest.display()))?;
        let children = node::list_children(db, Some(node.id), false, "").await?;
        for child in children {
            let child_dest = dest.join(&child.name);
            Box::pin(export_tree(db, store, &child, &child_dest)).await?;
        }
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| anyhow::anyhow!("create {}: {e}", parent.display()))?;
    }
    let bytes = read_vnode(store, node).await?;
    tokio::fs::write(dest, bytes)
        .await
        .map_err(|e| anyhow::anyhow!("write {}: {e}", dest.display()))?;
    Ok(())
}

async fn read_vnode(store: &DynFilestore, node: &VNode) -> anyhow::Result<Vec<u8>> {
    let path = node.file_path.as_deref().unwrap_or("");
    let download = store
        .open(path, &node.name)
        .await
        .map_err(|e| anyhow::anyhow!("open {}: {e}", node.name))?;
    let mut reader = download.reader;
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .await
        .map_err(|e| anyhow::anyhow!("read {}: {e}", node.name))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sea_orm::{ConnectionTrait, Database, EntityTrait, Schema};

    use super::*;
    use crate::preferences::default_preferences;
    use lariv_plugin_filesystem::{
        entities::filesystem_node::Entity as VNodeEntity,
        storage::{DynFilestore, LocalFilestore},
    };
    use lariv_plugin_website::entities::db_route::Entity as DbRouteEntity;

    fn expect_published(outcome: PublishOutcome) -> (usize, String) {
        match outcome {
            PublishOutcome::Published { files, prefix } => Some((files, prefix)),
            PublishOutcome::MissingSource | PublishOutcome::MissingSummary { .. } => None,
        }
        .expect("expected a published book")
    }

    async fn setup() -> (
        sea_orm::DatabaseConnection,
        Arc<DynFilestore>,
        tempfile::TempDir,
    ) {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        let schema = Schema::new(db.get_database_backend());
        db.execute(&schema.create_table_from_entity(VNodeEntity))
            .await
            .expect("filesystem table");
        db.execute(&schema.create_table_from_entity(DbRouteEntity))
            .await
            .expect("routes table");
        let dir = tempfile::tempdir().expect("store dir");
        let store: Arc<DynFilestore> = Arc::new(LocalFilestore::new(dir.path().to_string_lossy()));
        (db, store, dir)
    }

    async fn write_source(
        db: &DatabaseConnection,
        store: &DynFilestore,
        parent: Option<&VNode>,
        name: &str,
        directory: bool,
        body: Option<&str>,
    ) -> VNode {
        node::create(
            db,
            store,
            name.to_string(),
            directory,
            body.map(|text| NodeFile::Bytes {
                filename: name.to_string(),
                data: text.as_bytes().to_vec(),
            }),
            parent,
            None,
            None,
        )
        .await
        .expect("create vnode")
    }

    #[tokio::test]
    async fn publishes_a_book_onto_website_routes() {
        let (db, store, _dir) = setup().await;
        let root = write_source(&db, store.as_ref(), None, "mdbook", true, None).await;
        write_source(
            &db,
            store.as_ref(),
            Some(&root),
            "book.toml",
            false,
            Some("[book]\ntitle = \"Guide\"\n"),
        )
        .await;
        let src = write_source(&db, store.as_ref(), Some(&root), "src", true, None).await;
        write_source(
            &db,
            store.as_ref(),
            Some(&src),
            "SUMMARY.md",
            false,
            Some("# Summary\n\n- [Intro](./intro.md)\n"),
        )
        .await;
        write_source(
            &db,
            store.as_ref(),
            Some(&src),
            "intro.md",
            false,
            Some("# Intro\n\nHello from mdbook.\n"),
        )
        .await;

        let output = write_source(&db, store.as_ref(), None, "mdbook-site", true, None).await;

        let mut prefs = default_preferences();
        prefs.source_vnode_id = root.id;
        prefs.output_vnode_id = output.id;
        let outcome = publish(&db, store.as_ref(), &prefs).await.expect("publish");
        let (files, prefix) = expect_published(outcome);
        assert_eq!(prefix, "/book");
        assert!(files > 1, "built site should contain more than the index");

        let routes = DbRouteEntity::find().all(&db).await.expect("routes");
        assert_eq!(routes.len(), files + 1);
        let index = routes
            .iter()
            .find(|route| route.path == "/book/index.html")
            .expect("index route");
        assert_eq!(index.theme, RAW_FILE_THEME);
        let mut found_chapter = false;
        for route in routes.iter().filter(|route| route.path.ends_with(".html")) {
            let page = node::get_by_id(&db, route.page_id)
                .await
                .expect("page node")
                .expect("page exists");
            let html = String::from_utf8(read_vnode(store.as_ref(), &page).await.expect("bytes"))
                .expect("utf8");
            if html.contains("Hello from mdbook") {
                found_chapter = true;
            }
        }
        assert!(
            found_chapter,
            "chapter html should include the source markdown"
        );

        let css = routes.iter().any(|route| route.path.ends_with(".css"));
        assert!(css, "theme css should be published");

        let entry = routes
            .iter()
            .find(|route| route.path == "/book")
            .expect("prefix redirect");
        let entry_node = node::get_by_id(&db, entry.page_id)
            .await
            .expect("entry node")
            .expect("entry exists");
        let redirect = String::from_utf8(
            read_vnode(store.as_ref(), &entry_node)
                .await
                .expect("bytes"),
        )
        .expect("utf8");
        assert!(redirect.contains("/book/index.html"));

        let again = publish(&db, store.as_ref(), &prefs)
            .await
            .expect("republish");
        let (files, _) = expect_published(again);
        let count = DbRouteEntity::find().all(&db).await.expect("routes").len();
        assert_eq!(count, files + 1);
    }

    #[tokio::test]
    async fn empty_source_reports_the_missing_summary() {
        let (db, store, _dir) = setup().await;
        let root = write_source(&db, store.as_ref(), None, "mdbook", true, None).await;
        let output = write_source(&db, store.as_ref(), None, "mdbook-site", true, None).await;
        let mut prefs = default_preferences();
        prefs.source_vnode_id = root.id;
        prefs.output_vnode_id = output.id;
        let outcome = publish(&db, store.as_ref(), &prefs).await.expect("publish");
        assert_eq!(
            outcome,
            PublishOutcome::MissingSummary {
                source: "/mdbook".into(),
                relative: "src/SUMMARY.md".into(),
            }
        );
    }

    #[tokio::test]
    async fn missing_source_skips_the_build() {
        let (db, store, _dir) = setup().await;
        let outcome = publish(&db, store.as_ref(), &default_preferences())
            .await
            .expect("publish");
        assert_eq!(outcome, PublishOutcome::MissingSource);
    }
}
