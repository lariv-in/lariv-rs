//! Typed create-modal wiring for quotation swap keys.

use crate::keys::QuotationCreateModalKey;
use crate::routes::{QuotationCreateGetRouteTag, QuotationCreatePostRouteTag};

lariv_core::impl_create_modal!(
    QuotationCreateModalKey,
    QuotationCreateGetRouteTag,
    QuotationCreatePostRouteTag,
    "p_finance_quotations.QuotationCreateForm"
);
