use super::{
    handlers,
    keys::{TaxDeleteModalKey, TaxMultiSelectModalKey, TaxMultiSelectTableKey, TaxTableKey},
};

pub struct FinanceTaxesView;

pub struct FinanceTaxesMutate;

crate::define_plugin_routes! {
    plugin: FinanceTaxesTag;
    prefix: "/dashboard";
    routes: [
        get TaxDefaultRouteTag, "/finance-taxes", handlers::taxes::list, fragment(TaxTableKey), authorize(FinanceTaxesView, []);
        get TaxCreateGetRouteTag, "/finance-taxes/create", handlers::taxes::create_get, modal, authorize(FinanceTaxesMutate, []);
        post TaxCreatePostRouteTag, "/finance-taxes/create", handlers::taxes::create_post, authorize(FinanceTaxesMutate, []);
        get TaxDetailRouteTag, "/finance-taxes/t/{id}", handlers::taxes::detail, authorize(FinanceTaxesView, []);
        get TaxEditGetRouteTag, "/finance-taxes/t/{id}/edit", handlers::taxes::edit_get, modal, authorize(FinanceTaxesMutate, []);
        post TaxEditPostRouteTag, "/finance-taxes/t/{id}/edit", handlers::taxes::edit_post, authorize(FinanceTaxesMutate, []);
        get TaxDeleteGetRouteTag, "/finance-taxes/t/{id}/delete", handlers::taxes::delete_get, modal, authorize(FinanceTaxesMutate, []);
        post TaxDeletePostRouteTag, "/finance-taxes/t/{id}/delete", bare handlers::taxes::delete_post, fragment(TaxDeleteModalKey), authorize(FinanceTaxesMutate, []);
        get TaxMultiSelectRouteTag, "/finance-taxes/multi-select", handlers::taxes::multi_select, multi_select(TaxMultiSelectTableKey, TaxMultiSelectModalKey), authorize(FinanceTaxesView, []);
    ]
}
