use super::handlers;

lariv_core::define_plugin_routes! {
    plugin: SigningTag;
    prefix: "/dashboard";
    routes: [
        get SignatureCreateGetRouteTag, "/signing/signature", handlers::signature::create_get, modal;
        post SignatureCreatePostRouteTag, "/signing/signature", handlers::signature::create_post;
        post SignDocumentPostRouteTag, "/signing/documents/{id}/sign", handlers::sign_document::sign;
    ]
}
