//! Blog HTTP routes — tagged entries on [`lariv_core::http::HttpCapability`]'s route HList.

use lariv_core::define_plugin_routes;

use super::{
    handlers,
    keys::{
        BLOG_DELETE_MODAL, BLOG_TABLE, TAG_DELETE_MODAL, TAG_SELECT_MODAL, TAG_SELECT_TABLE,
        TAG_TABLE,
    },
};

define_plugin_routes! {
    plugin: BlogTag;
    prefix: "/dashboard";
    routes: [
        get BlogListRouteTag, "/blog", handlers::blogs::list, fragment(BLOG_TABLE);
        get BlogCreateGetRouteTag, "/blog/create", handlers::blogs::create_get, modal;
        post BlogCreatePostRouteTag, "/blog/create", handlers::blogs::create_post;
        get BlogDetailRouteTag, "/blog/p/{id}", handlers::blogs::detail;
        get BlogEditGetRouteTag, "/blog/p/{id}/edit", handlers::blogs::edit_get, modal;
        post BlogEditPostRouteTag, "/blog/p/{id}/edit", handlers::blogs::edit_post;
        get BlogDeleteGetRouteTag, "/blog/p/{id}/delete", handlers::blogs::delete_get, modal;
        post BlogDeletePostRouteTag, "/blog/p/{id}/delete", bare handlers::blogs::delete_post, fragment(BLOG_DELETE_MODAL);
        get BlogTagsListRouteTag, "/blog/tags", handlers::tags::list, fragment(TAG_TABLE);
        get BlogTagsSelectRouteTag, "/blog/tags/select", handlers::tags::select, multi_select(TAG_SELECT_TABLE, TAG_SELECT_MODAL);
        get BlogTagsCreateGetRouteTag, "/blog/tags/create", handlers::tags::create_get, modal;
        post BlogTagsCreatePostRouteTag, "/blog/tags/create", handlers::tags::create_post;
        get BlogTagsDetailRouteTag, "/blog/tags/{id}", handlers::tags::detail;
        get BlogTagsEditGetRouteTag, "/blog/tags/{id}/edit", handlers::tags::edit_get, modal;
        post BlogTagsEditPostRouteTag, "/blog/tags/{id}/edit", handlers::tags::edit_post;
        get BlogTagsDeleteGetRouteTag, "/blog/tags/{id}/delete", handlers::tags::delete_get, modal;
        post BlogTagsDeletePostRouteTag, "/blog/tags/{id}/delete", bare handlers::tags::delete_post, fragment(TAG_DELETE_MODAL);
    ]
}
