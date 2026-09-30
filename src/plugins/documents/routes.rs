use super::{
    handlers,
    keys::{DocumentDeleteModalKey, DocumentTableKey},
};

crate::define_plugin_routes! {
    plugin: DocumentsTag;
    prefix: "/dashboard";
    routes: [
        get DocumentDefaultRouteTag, "/documents", handlers::documents::list, fragment(DocumentTableKey);
        get DocumentCreateGetRouteTag, "/documents/create", handlers::documents::create_get, modal;
        post DocumentCreatePostRouteTag, "/documents/create", handlers::documents::create_post;
        get DocumentDetailRouteTag, "/documents/d/{id}", handlers::documents::detail;
        get DocumentEditGetRouteTag, "/documents/d/{id}/edit", handlers::documents::edit_get, modal;
        post DocumentEditPostRouteTag, "/documents/d/{id}/edit", handlers::documents::edit_post;
        get DocumentDeleteGetRouteTag, "/documents/d/{id}/delete", handlers::documents::delete_get, modal;
        post DocumentDeletePostRouteTag, "/documents/d/{id}/delete", bare handlers::documents::delete_post, fragment(DocumentDeleteModalKey);
    ]
}
