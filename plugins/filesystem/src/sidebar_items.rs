//! Extra entries in the filesystem sidebars.
//!
//! Addons call [`register`] while the app is mounting. [`root_items`] and
//! [`directory_items`] render those entries from the filesystem menus.

use std::sync::{Mutex, OnceLock};

use maud::{Markup, html};

/// One addon entry in the filesystem root sidebar and directory sidebars.
pub trait FilesystemSidebarItem: Send + Sync {
    fn root_item(&self, current_path: &str) -> Markup;
    fn directory_item(&self, directory_id: i64, active: &str) -> Markup;
    /// Path this root entry owns. A longer match beats built-in filesystem links
    /// such as "All Files", which would otherwise stay active on child paths.
    fn root_url(&self) -> Option<String> {
        None
    }
}

type SidebarItem = std::sync::Arc<dyn FilesystemSidebarItem>;
type ItemList = Vec<(&'static str, SidebarItem)>;

fn items() -> &'static Mutex<ItemList> {
    static ITEMS: OnceLock<Mutex<ItemList>> = OnceLock::new();
    ITEMS.get_or_init(|| Mutex::new(Vec::new()))
}

fn lock() -> std::sync::MutexGuard<'static, ItemList> {
    items()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Register `item` under `key`. A second registration of the same key is ignored.
pub fn register(key: &'static str, item: std::sync::Arc<dyn FilesystemSidebarItem>) {
    let mut items = lock();
    if items.iter().any(|(existing, _)| *existing == key) {
        return;
    }
    items.push((key, item));
}

/// URLs registered root entries own, used so they win active-state over shorter links.
pub fn root_urls() -> Vec<String> {
    let items = lock();
    items
        .iter()
        .filter_map(|(_, item)| item.root_url())
        .collect()
}

pub fn root_items(current_path: &str) -> Markup {
    let items = lock();
    let mut combined = Markup::default();
    for (_, item) in items.iter() {
        let piece = item.root_item(current_path);
        combined = html! {
            (combined)
            (piece)
        };
    }
    combined
}

pub fn directory_items(directory_id: i64, active: &str) -> Markup {
    let items = lock();
    let mut combined = Markup::default();
    for (_, item) in items.iter() {
        let piece = item.directory_item(directory_id, active);
        combined = html! {
            (combined)
            (piece)
        };
    }
    combined
}
