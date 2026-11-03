//! Compile-time HTMX swap keys for the blog plugin.

use lariv_core::components::SwapKey;

pub const BLOG_TABLE: SwapKey = SwapKey("blogs-table");
pub const TAG_TABLE: SwapKey = SwapKey("tags-table");
pub const TAG_SELECT_TABLE: SwapKey = SwapKey("tag-selection-table");
pub const BLOG_CREATE_MODAL: SwapKey = SwapKey("blog-create-modal");
pub const BLOG_EDIT_MODAL: SwapKey = SwapKey("blog-edit-modal");
pub const BLOG_DELETE_MODAL: SwapKey = SwapKey("blog-delete-modal");
pub const TAG_CREATE_MODAL: SwapKey = SwapKey("tag-create-modal");
pub const TAG_EDIT_MODAL: SwapKey = SwapKey("tag-edit-modal");
pub const TAG_DELETE_MODAL: SwapKey = SwapKey("tag-delete-modal");
pub const TAG_SELECT_MODAL: SwapKey = SwapKey("tag-selection-modal");
pub const BLOG_FK_AUTHOR: SwapKey = SwapKey("fk-blog-author");
pub const BLOG_FK_TAGS: SwapKey = SwapKey("fk-blog-tags");
