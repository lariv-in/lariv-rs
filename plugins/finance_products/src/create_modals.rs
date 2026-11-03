//! Typed [`CreateModal`] wiring for product swap keys.

use lariv_core::web::CreateModal;

use super::keys::PRODUCT_CREATE_MODAL;
use super::routes::{ProductCreateGetRouteTag, ProductCreatePostRouteTag};

pub const PRODUCT_CREATE: CreateModal<ProductCreateGetRouteTag, ProductCreatePostRouteTag> =
    CreateModal {
        key: PRODUCT_CREATE_MODAL,
        form_name: "p_finance_products.ProductCreateForm",
        get: ProductCreateGetRouteTag,
        post: ProductCreatePostRouteTag,
    };
