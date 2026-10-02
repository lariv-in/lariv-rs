use super::{
    handlers,
    keys::{ProductDeleteModalKey, ProductSelectModalKey, ProductSelectTableKey, ProductTableKey},
};

pub struct FinanceProductsView;

pub struct FinanceProductsMutate;

crate::define_plugin_routes! {
    plugin: FinanceProductsTag;
    prefix: "/dashboard";
    routes: [
        get ProductDefaultRouteTag, "/finance-products", handlers::products::list, fragment(ProductTableKey), authorize(FinanceProductsView, []);
        get ProductCreateGetRouteTag, "/finance-products/create", handlers::products::create_get, modal, authorize(FinanceProductsMutate, []);
        post ProductCreatePostRouteTag, "/finance-products/create", handlers::products::create_post, authorize(FinanceProductsMutate, []);
        get ProductDetailRouteTag, "/finance-products/p/{id}", handlers::products::detail, authorize(FinanceProductsView, []);
        get ProductEditGetRouteTag, "/finance-products/p/{id}/edit", handlers::products::edit_get, modal, authorize(FinanceProductsMutate, []);
        post ProductEditPostRouteTag, "/finance-products/p/{id}/edit", handlers::products::edit_post, authorize(FinanceProductsMutate, []);
        get ProductDeleteGetRouteTag, "/finance-products/p/{id}/delete", handlers::products::delete_get, modal, authorize(FinanceProductsMutate, []);
        post ProductDeletePostRouteTag, "/finance-products/p/{id}/delete", bare handlers::products::delete_post, fragment(ProductDeleteModalKey), authorize(FinanceProductsMutate, []);
        get ProductFkSelectRouteTag, "/finance-products/pick-product", handlers::products::select, fk_select(ProductSelectTableKey, ProductSelectModalKey), authorize(FinanceProductsView, []);
    ]
}
