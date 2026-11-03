//! Typed [`CreateModal`] wiring for holiday swap keys.

use lariv_core::web::CreateModal;

use super::keys::{HOLIDAY_CREATE_MODAL, HOLIDAY_IMPORT_MODAL};
use super::routes::{
    HolidayCreateGetRouteTag, HolidayCreatePostRouteTag, HolidayImportGetRouteTag,
    HolidayImportPostRouteTag,
};

pub const HOLIDAY_CREATE: CreateModal<HolidayCreateGetRouteTag, HolidayCreatePostRouteTag> =
    CreateModal {
        key: HOLIDAY_CREATE_MODAL,
        form_name: "p_hr.HolidayCreateForm",
        get: HolidayCreateGetRouteTag,
        post: HolidayCreatePostRouteTag,
    };

pub const HOLIDAY_IMPORT: CreateModal<HolidayImportGetRouteTag, HolidayImportPostRouteTag> =
    CreateModal {
        key: HOLIDAY_IMPORT_MODAL,
        form_name: "p_hr.HolidayImportForm",
        get: HolidayImportGetRouteTag,
        post: HolidayImportPostRouteTag,
    };
