//! Copy filesystem VNode file bytes into the Typst work directory for `#image(...)`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sea_orm::{DatabaseConnection, EntityTrait};
use crate::plugins::filesystem::{
    entities::filesystem_node::Entity as VNodeEntity,
    storage::DynFilestore,
    zip::read_file_bytes,
};

/// Context for resolving `vnodeImage(...)` in invoice PDF Minijinja templates.
#[derive(Clone)]
pub struct VnodeImageContext {
    db: DatabaseConnection,
    store: Arc<DynFilestore>,
    asset_dir: PathBuf,
    cache: Arc<Mutex<HashMap<i64, String>>>,
}

impl VnodeImageContext {
    pub fn new(
        db: DatabaseConnection,
        store: Arc<DynFilestore>,
        asset_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            store,
            asset_dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Resolve a VNode id to a local filename under the Typst work directory.
    pub fn resolve_sync(&self, vnode_id: i64) -> Result<String, String> {
        if vnode_id <= 0 {
            return Err("vnodeImage: invalid VNode id".into());
        }
        if let Some(name) = self.cache.lock().unwrap().get(&vnode_id) {
            return Ok(name.clone());
        }
        let db = self.db.clone();
        let store = Arc::clone(&self.store);
        let asset_dir = self.asset_dir.clone();
        let filename = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                write_vnode_image(vnode_id, &db, store.as_ref(), &asset_dir).await
            })
        })?;
        self.cache
            .lock()
            .unwrap()
            .insert(vnode_id, filename.clone());
        Ok(filename)
    }
}

async fn write_vnode_image(
    vnode_id: i64,
    db: &DatabaseConnection,
    store: &DynFilestore,
    asset_dir: &Path,
) -> Result<String, String> {
    let node = VNodeEntity::find_by_id(vnode_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("vnodeImage: VNode {vnode_id} not found"))?;
    if node.is_directory {
        return Err(format!("vnodeImage: VNode {vnode_id} is a directory"));
    }
    let bytes = read_file_bytes(store, &node)
        .await
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Err(format!("vnodeImage: VNode {vnode_id} is empty"));
    }
    std::fs::create_dir_all(asset_dir).map_err(|e| e.to_string())?;
    let ext = Path::new(&node.name)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    let filename = format!("vnode-{vnode_id}{ext}");
    std::fs::write(asset_dir.join(&filename), &bytes)
        .map_err(|e| format!("vnodeImage: write {filename}: {e}"))?;
    Ok(filename)
}
