//! Typed [`CreateModal`] wiring for holiday swap keys.

use super::keys::{HolidayCreateModalKey, HolidayImportModalKey};
use super::routes::{
    HolidayCreateGetRouteTag, HolidayCreatePostRouteTag, HolidayImportGetRouteTag,
    HolidayImportPostRouteTag,
};

lariv_core::impl_create_modal!(
    HolidayCreateModalKey,
    HolidayCreateGetRouteTag,
    HolidayCreatePostRouteTag,
    "p_hr.HolidayCreateForm"
);
lariv_core::impl_create_modal!(
    HolidayImportModalKey,
    HolidayImportGetRouteTag,
    HolidayImportPostRouteTag,
    "p_hr.HolidayImportForm"
);
