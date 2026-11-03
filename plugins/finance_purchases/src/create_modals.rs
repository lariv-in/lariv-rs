//! Picker modal keys are plain [`SwapKey`] consts; pairing lives at `fk_select`.

use lariv_core::web::CreateModal;

use super::keys::VENDOR_CREATE_MODAL;
use super::routes::{VendorCreateGetRouteTag, VendorCreatePostRouteTag};

pub const VENDOR_CREATE: CreateModal<VendorCreateGetRouteTag, VendorCreatePostRouteTag> =
    CreateModal {
        key: VENDOR_CREATE_MODAL,
        form_name: "p_finance_purchases.VendorCreateForm",
        get: VendorCreateGetRouteTag,
        post: VendorCreatePostRouteTag,
    };
