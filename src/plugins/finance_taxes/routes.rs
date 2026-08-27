use super::{
    handlers,
    keys::{TaxDeleteModalKey, TaxMultiSelectModalKey, TaxMultiSelectTableKey, TaxTableKey},
};

crate::define_plugin_routes! {
    plugin: FinanceTaxesTag;
    routes: [
        get TaxDefaultRouteTag, "/finance-taxes", handlers::taxes::list, fragment(TaxTableKey);
        get TaxCreateGetRouteTag, "/finance-taxes/create", handlers::taxes::create_get, modal;
        post TaxCreatePostRouteTag, "/finance-taxes/create", handlers::taxes::create_post;
        get TaxDetailRouteTag, "/finance-taxes/t/{id}", handlers::taxes::detail;
        get TaxEditGetRouteTag, "/finance-taxes/t/{id}/edit", handlers::taxes::edit_get, modal;
        post TaxEditPostRouteTag, "/finance-taxes/t/{id}/edit", handlers::taxes::edit_post;
        get TaxDeleteGetRouteTag, "/finance-taxes/t/{id}/delete", handlers::taxes::delete_get, modal;
        post TaxDeletePostRouteTag, "/finance-taxes/t/{id}/delete", bare handlers::taxes::delete_post, fragment(TaxDeleteModalKey);
        get TaxMultiSelectRouteTag, "/finance-taxes/multi-select", handlers::taxes::multi_select, multi_select(TaxMultiSelectTableKey, TaxMultiSelectModalKey);
    ]
}
