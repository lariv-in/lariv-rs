use super::keys::{
    FormCreateModalKey, FormResponseCreateModalKey, FormSelectModalKey, FormSelectTableKey,
};
use super::routes::{
    FormCreateGetRouteTag, FormCreatePostRouteTag, FormResponseCreateGetRouteTag,
    FormResponseCreatePostRouteTag,
};

crate::impl_create_modal!(
    FormCreateModalKey,
    FormCreateGetRouteTag,
    FormCreatePostRouteTag,
    "p_forms.FormCreateForm"
);
crate::impl_picker_modal!(FormSelectModalKey, FormSelectTableKey);

crate::impl_create_modal!(
    FormResponseCreateModalKey,
    FormResponseCreateGetRouteTag,
    FormResponseCreatePostRouteTag,
    "p_forms.FormResponseCreateForm"
);
