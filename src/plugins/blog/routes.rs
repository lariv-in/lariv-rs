//! Blog HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::plugin_routes::define_plugin_routes;

use super::{
    handlers,
    templates::{
        BlogConfirmDeletePageTag, BlogDetailPage, BlogDetailPageTag, BlogFormPage, BlogFormPageTag,
        BlogListPage, BlogListPageTag, ConfirmDeletePage, TagDetailPage, TagDetailPageTag,
        TagFormPage, TagFormPageTag, TagListPage, TagListPageTag, TagSelectPage, TagSelectPageTag,
    },
};

define_plugin_routes! {
    plugin: BlogTag;
    proof: BlogRoutesProof;
    pages: [
        pane BlogListIdx, BlogListP => BlogListPageTag, BlogListPage;
        pane BlogFormIdx, BlogFormP => BlogFormPageTag, BlogFormPage;
        pane BlogDetailIdx, BlogDetailP => BlogDetailPageTag, BlogDetailPage;
        page ConfirmDeleteIdx, ConfirmDeleteP => BlogConfirmDeletePageTag, ConfirmDeletePage;
        pane TagListIdx, TagListP => TagListPageTag, TagListPage;
        pane TagFormIdx, TagFormP => TagFormPageTag, TagFormPage;
        pane TagDetailIdx, TagDetailP => TagDetailPageTag, TagDetailPage;
        page TagSelectIdx, TagSelectP => TagSelectPageTag, TagSelectPage;
    ];
    routes: [
        get BlogListRouteTag, "/blog", handlers::blogs::list;
        get BlogCreateGetRouteTag, "/blog/create", handlers::blogs::create_get;
        post BlogCreatePostRouteTag, "/blog/create", handlers::blogs::create_post;
        get BlogDetailRouteTag, "/blog/p/{id}", handlers::blogs::detail;
        get BlogEditGetRouteTag, "/blog/p/{id}/edit", handlers::blogs::edit_get;
        post BlogEditPostRouteTag, "/blog/p/{id}/edit", handlers::blogs::edit_post;
        get BlogDeleteGetRouteTag, "/blog/p/{id}/delete", handlers::blogs::delete_get;
        post BlogDeletePostRouteTag, "/blog/p/{id}/delete", bare handlers::blogs::delete_post;
        get BlogTagsListRouteTag, "/blog/tags", handlers::tags::list;
        get BlogTagsSelectRouteTag, "/blog/tags/select", handlers::tags::select;
        get BlogTagsCreateGetRouteTag, "/blog/tags/create", handlers::tags::create_get;
        post BlogTagsCreatePostRouteTag, "/blog/tags/create", handlers::tags::create_post;
        get BlogTagsDetailRouteTag, "/blog/tags/{id}", handlers::tags::detail;
        get BlogTagsEditGetRouteTag, "/blog/tags/{id}/edit", handlers::tags::edit_get;
        post BlogTagsEditPostRouteTag, "/blog/tags/{id}/edit", handlers::tags::edit_post;
        get BlogTagsDeleteGetRouteTag, "/blog/tags/{id}/delete", handlers::tags::delete_get;
        post BlogTagsDeletePostRouteTag, "/blog/tags/{id}/delete", bare handlers::tags::delete_post;
    ]
}
