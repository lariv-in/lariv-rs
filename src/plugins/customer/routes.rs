use super::{
    handlers,
    keys::{
        CustomerDeleteModalKey, CustomerSelectModalKey, CustomerSelectTableKey, CustomerTableKey,
    },
};

pub struct CustomerView;

pub struct CustomerMutate;

crate::define_plugin_routes! {
    plugin: CustomerTag;
    prefix: "/dashboard";
    routes: [
        get CustomerDefaultRouteTag, "/customers", handlers::customers::list, fragment(CustomerTableKey), authorize(CustomerView, []);
        get CustomerCreateGetRouteTag, "/customers/create", handlers::customers::create_get, modal, authorize(CustomerMutate, []);
        post CustomerCreatePostRouteTag, "/customers/create", handlers::customers::create_post, authorize(CustomerMutate, []);
        get CustomerDetailRouteTag, "/customers/c/{id}", handlers::customers::detail, authorize(CustomerView, []);
        get CustomerEditGetRouteTag, "/customers/c/{id}/edit", handlers::customers::edit_get, modal, authorize(CustomerMutate, []);
        post CustomerEditPostRouteTag, "/customers/c/{id}/edit", handlers::customers::edit_post, authorize(CustomerMutate, []);
        get CustomerDeleteGetRouteTag, "/customers/c/{id}/delete", handlers::customers::delete_get, modal, authorize(CustomerMutate, []);
        post CustomerDeletePostRouteTag, "/customers/c/{id}/delete", bare handlers::customers::delete_post, fragment(CustomerDeleteModalKey), authorize(CustomerMutate, []);
        get CustomerFkSelectRouteTag, "/customers/pick-customer", handlers::customers::select, fk_select(CustomerSelectTableKey, CustomerSelectModalKey), authorize(CustomerView, []);
    ]
}
