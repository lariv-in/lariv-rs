//! Typed [`CreateModal`] / [`PickerModal`] wiring for CRM swap keys.

use crate::picker::PickerModal;
use crate::web::CreateModal;

use super::keys::{
    CompanyCreateModalKey, CompanySelectModalKey, CompanySelectTableKey, ContactCreateModalKey,
    ContactSelectModalKey, ContactSelectTableKey, LeadCreateModalKey,
};
use super::routes::{
    CompanyCreateGetRouteTag, CompanyCreatePostRouteTag, ContactCreateGetRouteTag,
    ContactCreatePostRouteTag, LeadCreateGetRouteTag, LeadCreatePostRouteTag,
};

macro_rules! crm_create_modal {
    ($modal:ident, $get:ty, $post:ty, $form:expr) => {
        impl CreateModal for $modal {
            type Get = $get;
            type Post = $post;
            const FORM_NAME: &'static str = $form;
        }
    };
}

macro_rules! crm_picker_modal {
    ($modal:ident, $table:ty) => {
        impl PickerModal for $modal {
            type Table = $table;
        }
    };
}

crm_create_modal!(
    LeadCreateModalKey,
    LeadCreateGetRouteTag,
    LeadCreatePostRouteTag,
    "p_crm.LeadCreateForm"
);
crm_create_modal!(
    CompanyCreateModalKey,
    CompanyCreateGetRouteTag,
    CompanyCreatePostRouteTag,
    "p_crm.CompanyCreateForm"
);
crm_create_modal!(
    ContactCreateModalKey,
    ContactCreateGetRouteTag,
    ContactCreatePostRouteTag,
    "p_crm.ContactCreateForm"
);

crm_picker_modal!(CompanySelectModalKey, CompanySelectTableKey);
crm_picker_modal!(ContactSelectModalKey, ContactSelectTableKey);
