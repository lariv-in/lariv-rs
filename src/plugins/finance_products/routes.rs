use super::{
    handlers,
    keys::{ProductSelectModalKey, ProductSelectTableKey, ProductTableKey},
};

crate::define_plugin_routes! {
    plugin: FinanceProductsTag;
    routes: [
        get ProductDefaultRouteTag, "/finance-products", handlers::products::list, fragment(ProductTableKey);
        get ProductCreateGetRouteTag, "/finance-products/create", handlers::products::create_get, modal;
        post ProductCreatePostRouteTag, "/finance-products/create", handlers::products::create_post;
        get ProductDetailRouteTag, "/finance-products/p/{id}", handlers::products::detail;
        get ProductEditGetRouteTag, "/finance-products/p/{id}/edit", handlers::products::edit_get, modal;
        post ProductEditPostRouteTag, "/finance-products/p/{id}/edit", handlers::products::edit_post;
        post ProductDeletePostRouteTag, "/finance-products/p/{id}/delete", bare handlers::products::delete_post, redirect;
        get ProductFkSelectRouteTag, "/finance-products/pick-product", handlers::products::select, fk_select(ProductSelectTableKey, ProductSelectModalKey);
    ]
}
