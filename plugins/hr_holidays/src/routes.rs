use super::{
    handlers,
    keys::{HOLIDAY_DELETE_MODAL, HOLIDAY_TABLE},
};

pub struct HolidayView;

pub struct HolidayMutate;

/// Pull India.gov calendar holidays into the holidays table. Empty allowlist: superuser only.
pub struct HolidayImport;

lariv_core::define_plugin_routes! {
    plugin: HrHolidaysTag;
    prefix: "/dashboard";
    routes: [
        get HolidayListRouteTag, "/hr/holidays", handlers::holidays::list, fragment(HOLIDAY_TABLE), authorize(HolidayView, []);
        get HolidayCreateGetRouteTag, "/hr/holidays/create", handlers::holidays::create_get, modal, authorize(HolidayMutate, []);
        post HolidayCreatePostRouteTag, "/hr/holidays/create", handlers::holidays::create_post, authorize(HolidayMutate, []);
        get HolidayImportGetRouteTag, "/hr/holidays/import", handlers::holidays::import_get, modal, authorize(HolidayImport, []);
        post HolidayImportPostRouteTag, "/hr/holidays/import", handlers::holidays::import_post, authorize(HolidayImport, []);
        get HolidayDetailRouteTag, "/hr/holidays/{id}", handlers::holidays::detail, authorize(HolidayView, []);
        get HolidayEditGetRouteTag, "/hr/holidays/{id}/edit", handlers::holidays::edit_get, modal, authorize(HolidayMutate, []);
        post HolidayEditPostRouteTag, "/hr/holidays/{id}/edit", handlers::holidays::edit_post, authorize(HolidayMutate, []);
        get HolidayDeleteGetRouteTag, "/hr/holidays/{id}/delete", handlers::holidays::delete_get, modal, authorize(HolidayMutate, []);
        post HolidayDeletePostRouteTag, "/hr/holidays/{id}/delete", bare handlers::holidays::delete_post, fragment(HOLIDAY_DELETE_MODAL), authorize(HolidayMutate, []);
    ]
}
