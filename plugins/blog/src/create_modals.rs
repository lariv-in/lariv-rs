//! Typed [`CreateModal`] wiring for blog tag swap keys.

use lariv_core::web::CreateModal;

use super::keys::TAG_CREATE_MODAL;
use super::routes::{BlogTagsCreateGetRouteTag, BlogTagsCreatePostRouteTag};

pub const TAG_CREATE: CreateModal<BlogTagsCreateGetRouteTag, BlogTagsCreatePostRouteTag> =
    CreateModal {
        key: TAG_CREATE_MODAL,
        form_name: "p_blog.TagCreateForm",
        get: BlogTagsCreateGetRouteTag,
        post: BlogTagsCreatePostRouteTag,
    };
