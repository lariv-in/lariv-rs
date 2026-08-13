//! Blog HTTP routes — tagged entries on [`crate::http::HttpCapability`]'s route HList.

use crate::define_plugin_routes;

use super::{
    handlers,
    keys::{
        BlogDeleteModalKey, BlogTableKey, TagDeleteModalKey, TagSelectModalKey, TagSelectTableKey,
        TagTableKey,
    },
};

define_plugin_routes! {
    plugin: BlogTag;
    routes: [
        get BlogListRouteTag, "/blog", handlers::blogs::list, fragment(BlogTableKey);
        get BlogCreateGetRouteTag, "/blog/create", handlers::blogs::create_get, modal;
        post BlogCreatePostRouteTag, "/blog/create", handlers::blogs::create_post;
        get BlogDetailRouteTag, "/blog/p/{id}", handlers::blogs::detail;
        get BlogEditGetRouteTag, "/blog/p/{id}/edit", handlers::blogs::edit_get, modal;
        post BlogEditPostRouteTag, "/blog/p/{id}/edit", handlers::blogs::edit_post;
        get BlogDeleteGetRouteTag, "/blog/p/{id}/delete", handlers::blogs::delete_get, modal;
        post BlogDeletePostRouteTag, "/blog/p/{id}/delete", bare handlers::blogs::delete_post, fragment(BlogDeleteModalKey);
        get BlogTagsListRouteTag, "/blog/tags", handlers::tags::list, fragment(TagTableKey);
        get BlogTagsSelectRouteTag, "/blog/tags/select", handlers::tags::select, multi_select(TagSelectTableKey, TagSelectModalKey);
        get BlogTagsCreateGetRouteTag, "/blog/tags/create", handlers::tags::create_get, modal;
        post BlogTagsCreatePostRouteTag, "/blog/tags/create", handlers::tags::create_post;
        get BlogTagsDetailRouteTag, "/blog/tags/{id}", handlers::tags::detail;
        get BlogTagsEditGetRouteTag, "/blog/tags/{id}/edit", handlers::tags::edit_get, modal;
        post BlogTagsEditPostRouteTag, "/blog/tags/{id}/edit", handlers::tags::edit_post;
        get BlogTagsDeleteGetRouteTag, "/blog/tags/{id}/delete", handlers::tags::delete_get, modal;
        post BlogTagsDeletePostRouteTag, "/blog/tags/{id}/delete", bare handlers::tags::delete_post, fragment(TagDeleteModalKey);
    ]
}
