//! VNode business logic — go` / `db_fs.go`.

pub mod access;
pub mod error;
pub mod mutate;
pub mod query;
pub mod text;

pub use access::{
    authorize_change_file, authorize_create_in, authorize_open, authorize_remove,
    authorize_set_access, authorize_set_root_access, authorize_view, authorize_view_root,
    authorize_view_tree,
};
pub use error::{NodeError, NodeFile};
pub use mutate::{
    create, create_owned, delete_direct_children, delete_tree, ensure_directory_path, move_to,
    set_access, set_access_all, update,
};
pub use query::{
    apply_listing_parent, children_count, count_visible_children, file_size_display, find_child,
    get_by_id, get_by_path, get_path, list_children,
};
pub use text::{
    MAX_TEXT_BYTES, decode_text, editor_language, ext_of, is_typst_file, item_type,
    require_text_content_size, sanitize_node_name, try_read_text,
};
