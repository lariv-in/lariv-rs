//! Typed [`CreateModal`] / [`PickerModal`] wiring for blog tag swap keys.

use super::keys::{TagCreateModalKey, TagSelectModalKey, TagSelectTableKey};
use super::routes::{BlogTagsCreateGetRouteTag, BlogTagsCreatePostRouteTag};

crate::impl_create_modal!(
    TagCreateModalKey,
    BlogTagsCreateGetRouteTag,
    BlogTagsCreatePostRouteTag,
    "p_blog.TagCreateForm"
);
crate::impl_picker_modal!(TagSelectModalKey, TagSelectTableKey);
