use super::{
    handlers,
    keys::{DocumentDeleteModalKey, DocumentSelectModalKey, DocumentSelectTableKey, DocumentTableKey},
};

crate::define_plugin_routes! {
    plugin: DocumentsTag;
    prefix: "/dashboard";
    routes: [
        get DocumentDefaultRouteTag, "/documents", handlers::documents::list, fragment(DocumentTableKey);
        get DocumentPrefsGetRouteTag, "/documents/preferences", handlers::preferences::get;
        post DocumentPrefsPostRouteTag, "/documents/preferences", handlers::preferences::post;
        get DocumentCreateGetRouteTag, "/documents/create", handlers::documents::create_get, modal;
        post DocumentCreatePostRouteTag, "/documents/create", handlers::documents::create_post;
        get DocumentDetailRouteTag, "/documents/d/{id}", handlers::documents::detail;
        get DocumentEditGetRouteTag, "/documents/d/{id}/edit", handlers::documents::edit_get, modal;
        post DocumentEditPostRouteTag, "/documents/d/{id}/edit", handlers::documents::edit_post;
        get DocumentAadharSelectRouteTag, "/documents/pick/aadhar", handlers::documents::select_aadhar, fk_select(DocumentSelectTableKey, DocumentSelectModalKey);
        get DocumentPanSelectRouteTag, "/documents/pick/pan", handlers::documents::select_pan, fk_select(DocumentSelectTableKey, DocumentSelectModalKey);
        get DocumentPassportSelectRouteTag, "/documents/pick/passport", handlers::documents::select_passport, fk_select(DocumentSelectTableKey, DocumentSelectModalKey);
        get DocumentDeleteGetRouteTag, "/documents/d/{id}/delete", handlers::documents::delete_get, modal;
        post DocumentDeletePostRouteTag, "/documents/d/{id}/delete", bare handlers::documents::delete_post, fragment(DocumentDeleteModalKey);
    ]
}
