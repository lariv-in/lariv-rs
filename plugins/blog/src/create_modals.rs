//! Typed [`CreateModal`] / [`PickerModal`] wiring for blog tag swap keys.

use super::keys::{TagCreateModalKey, TagSelectModalKey, TagSelectTableKey};
use super::routes::{BlogTagsCreateGetRouteTag, BlogTagsCreatePostRouteTag};

lariv_core::impl_create_modal!(
    TagCreateModalKey,
    BlogTagsCreateGetRouteTag,
    BlogTagsCreatePostRouteTag,
    "p_blog.TagCreateForm"
);
lariv_core::impl_picker_modal!(TagSelectModalKey, TagSelectTableKey);
