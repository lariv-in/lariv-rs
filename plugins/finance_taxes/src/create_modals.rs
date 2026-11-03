//! Typed [`CreateModal`] wiring for tax swap keys.

use lariv_core::web::CreateModal;

use super::keys::TAX_CREATE_MODAL;
use super::routes::{TaxCreateGetRouteTag, TaxCreatePostRouteTag};

pub const TAX_CREATE: CreateModal<TaxCreateGetRouteTag, TaxCreatePostRouteTag> = CreateModal {
    key: TAX_CREATE_MODAL,
    form_name: "p_taxes.TaxCreateForm",
    get: TaxCreateGetRouteTag,
    post: TaxCreatePostRouteTag,
};
