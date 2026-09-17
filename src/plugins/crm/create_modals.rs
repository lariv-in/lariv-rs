//! Typed [`CreateModal`] / [`PickerModal`] wiring for CRM swap keys.

use crate::picker::PickerModal;
use crate::web::CreateModal;

use super::keys::{
    LeadCreateModalKey, LeadTagCreateModalKey, LeadTagSelectModalKey, LeadTagSelectTableKey,
    TaskCreateModalKey,
};
use super::routes::{
    LeadCreateGetRouteTag, LeadCreatePostRouteTag, LeadTagCreateGetRouteTag,
    LeadTagCreatePostRouteTag, TaskCreateGetRouteTag, TaskCreatePostRouteTag,
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
    TaskCreateModalKey,
    TaskCreateGetRouteTag,
    TaskCreatePostRouteTag,
    "p_crm.TaskCreateForm"
);
crm_create_modal!(
    LeadTagCreateModalKey,
    LeadTagCreateGetRouteTag,
    LeadTagCreatePostRouteTag,
    "p_crm.LeadTagCreateForm"
);

crm_picker_modal!(LeadTagSelectModalKey, LeadTagSelectTableKey);
