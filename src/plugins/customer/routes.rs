use super::{
    handlers,
    keys::{CustomerSelectModalKey, CustomerSelectTableKey, CustomerTableKey},
};

crate::define_plugin_routes! {
    plugin: CustomerTag;
    routes: [
        get CustomerDefaultRouteTag, "/customers", handlers::customers::list, fragment(CustomerTableKey);
        get CustomerCreateGetRouteTag, "/customers/create", handlers::customers::create_get, modal;
        post CustomerCreatePostRouteTag, "/customers/create", handlers::customers::create_post;
        get CustomerDetailRouteTag, "/customers/c/{id}", handlers::customers::detail;
        get CustomerEditGetRouteTag, "/customers/c/{id}/edit", handlers::customers::edit_get;
        post CustomerEditPostRouteTag, "/customers/c/{id}/edit", handlers::customers::edit_post;
        post CustomerDeletePostRouteTag, "/customers/c/{id}/delete", bare handlers::customers::delete_post, redirect;
        get CustomerFkSelectRouteTag, "/customers/pick-customer", handlers::customers::select, fk_select(CustomerSelectTableKey, CustomerSelectModalKey);
    ]
}
