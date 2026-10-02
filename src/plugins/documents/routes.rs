use super::{
    handlers,
    keys::{
        DocumentDeleteModalKey, DocumentSelectModalKey, DocumentSelectTableKey, DocumentTableKey,
    },
};

pub struct DocumentsView;

pub struct DocumentsMutate;

crate::define_plugin_routes! {
    plugin: DocumentsTag;
    prefix: "/dashboard";
    routes: [
        get DocumentDefaultRouteTag, "/documents", handlers::documents::list, fragment(DocumentTableKey), authorize(DocumentsView, []);
        get DocumentPrefsGetRouteTag, "/documents/preferences", handlers::preferences::get, authorize(DocumentsView, []);
        post DocumentPrefsPostRouteTag, "/documents/preferences", handlers::preferences::post, authorize(DocumentsMutate, []);
        get DocumentCreateGetRouteTag, "/documents/create", handlers::documents::create_get, modal, authorize(DocumentsMutate, []);
        post DocumentCreatePostRouteTag, "/documents/create", handlers::documents::create_post, authorize(DocumentsMutate, []);
        get DocumentDetailRouteTag, "/documents/d/{id}", handlers::documents::detail, authorize(DocumentsView, []);
        get DocumentEditGetRouteTag, "/documents/d/{id}/edit", handlers::documents::edit_get, modal, authorize(DocumentsMutate, []);
        post DocumentEditPostRouteTag, "/documents/d/{id}/edit", handlers::documents::edit_post, authorize(DocumentsMutate, []);
        get DocumentAadharSelectRouteTag, "/documents/pick/aadhar", handlers::documents::select_aadhar, fk_select(DocumentSelectTableKey, DocumentSelectModalKey), authorize(DocumentsView, []);
        get DocumentPanSelectRouteTag, "/documents/pick/pan", handlers::documents::select_pan, fk_select(DocumentSelectTableKey, DocumentSelectModalKey), authorize(DocumentsView, []);
        get DocumentPassportSelectRouteTag, "/documents/pick/passport", handlers::documents::select_passport, fk_select(DocumentSelectTableKey, DocumentSelectModalKey), authorize(DocumentsView, []);
        get DocumentDeleteGetRouteTag, "/documents/d/{id}/delete", handlers::documents::delete_get, modal, authorize(DocumentsMutate, []);
        post DocumentDeletePostRouteTag, "/documents/d/{id}/delete", bare handlers::documents::delete_post, fragment(DocumentDeleteModalKey), authorize(DocumentsMutate, []);
    ]
}
