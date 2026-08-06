//! Compile-time HTMX swap keys for the blog plugin.

use crate::swap_key;

swap_key!(BlogTableKey, "blogs-table");
swap_key!(TagTableKey, "tags-table");
swap_key!(TagSelectTableKey, "tag-selection-table");
swap_key!(BlogCreateModalKey, "blog-create-modal");
swap_key!(BlogDeleteModalKey, "blog-delete-modal");
swap_key!(TagCreateModalKey, "tag-create-modal");
swap_key!(TagDeleteModalKey, "tag-delete-modal");
swap_key!(TagSelectModalKey, "tag-selection-modal");
swap_key!(BlogFkAuthorKey, "fk-blog-author");
swap_key!(BlogFkTagsKey, "fk-blog-tags");
