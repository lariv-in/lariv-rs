use super::{
    handlers,
    keys::{ServiceDeleteModalKey, ServiceSelectModalKey, ServiceSelectTableKey, ServiceTableKey},
};

pub struct FinanceServicesView;

pub struct FinanceServicesMutate;

lariv_core::define_plugin_routes! {
    plugin: FinanceServicesTag;
    prefix: "/dashboard";
    routes: [
        get ServiceDefaultRouteTag, "/finance-services", handlers::services::list, fragment(ServiceTableKey), authorize(FinanceServicesView, []);
        get ServiceCreateGetRouteTag, "/finance-services/create", handlers::services::create_get, modal, authorize(FinanceServicesMutate, []);
        post ServiceCreatePostRouteTag, "/finance-services/create", handlers::services::create_post, authorize(FinanceServicesMutate, []);
        get ServiceDetailRouteTag, "/finance-services/p/{id}", handlers::services::detail, authorize(FinanceServicesView, []);
        get ServiceEditGetRouteTag, "/finance-services/p/{id}/edit", handlers::services::edit_get, modal, authorize(FinanceServicesMutate, []);
        post ServiceEditPostRouteTag, "/finance-services/p/{id}/edit", handlers::services::edit_post, authorize(FinanceServicesMutate, []);
        get ServiceDeleteGetRouteTag, "/finance-services/p/{id}/delete", handlers::services::delete_get, modal, authorize(FinanceServicesMutate, []);
        post ServiceDeletePostRouteTag, "/finance-services/p/{id}/delete", bare handlers::services::delete_post, fragment(ServiceDeleteModalKey), authorize(FinanceServicesMutate, []);
        get ServiceFkSelectRouteTag, "/finance-services/pick-service", handlers::services::select, fk_select(ServiceSelectTableKey, ServiceSelectModalKey), authorize(FinanceServicesView, []);
    ]
}
