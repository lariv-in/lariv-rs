use super::{
    handlers,
    keys::{FiscalYearSelectModalKey, FiscalYearSelectTableKey, FiscalYearTableKey},
};

crate::define_plugin_routes! {
    plugin: FinanceFiscalYearTag;
    routes: [
        get FiscalYearDefaultRouteTag, "/finance-fiscal-years", handlers::fiscal_years::list, fragment(FiscalYearTableKey);
        get FiscalYearCreateGetRouteTag, "/finance-fiscal-years/create", handlers::fiscal_years::create_get, modal;
        post FiscalYearCreatePostRouteTag, "/finance-fiscal-years/create", handlers::fiscal_years::create_post;
        get FiscalYearDetailRouteTag, "/finance-fiscal-years/fy/{id}", handlers::fiscal_years::detail;
        get FiscalYearEditGetRouteTag, "/finance-fiscal-years/fy/{id}/edit", handlers::fiscal_years::edit_get, modal;
        post FiscalYearEditPostRouteTag, "/finance-fiscal-years/fy/{id}/edit", handlers::fiscal_years::edit_post;
        post FiscalYearDeletePostRouteTag, "/finance-fiscal-years/fy/{id}/delete", bare handlers::fiscal_years::delete_post, redirect;
        get FiscalYearSelectRouteTag, "/finance-fiscal-years/select", handlers::fiscal_years::select, fk_select(FiscalYearSelectTableKey, FiscalYearSelectModalKey);
    ]
}
