use lariv_core::web::CreateModal;

use crate::keys::{FORM_CREATE_MODAL, FORM_RESPONSE_CREATE_MODAL};
use crate::routes::{
    FormCreateGetRouteTag, FormCreatePostRouteTag, FormResponseCreateGetRouteTag,
    FormResponseCreatePostRouteTag,
};

pub const FORM_CREATE: CreateModal<FormCreateGetRouteTag, FormCreatePostRouteTag> = CreateModal {
    key: FORM_CREATE_MODAL,
    form_name: "p_forms.FormCreateForm",
    get: FormCreateGetRouteTag,
    post: FormCreatePostRouteTag,
};

pub const FORM_RESPONSE_CREATE: CreateModal<
    FormResponseCreateGetRouteTag,
    FormResponseCreatePostRouteTag,
> = CreateModal {
    key: FORM_RESPONSE_CREATE_MODAL,
    form_name: "p_forms.FormResponseCreateForm",
    get: FormResponseCreateGetRouteTag,
    post: FormResponseCreatePostRouteTag,
};

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_core::components::picker::picker_create_button;

    #[test]
    fn form_picker_create_button_embeds_target_input() {
        let html = picker_create_button(
            FORM_CREATE,
            "form_id",
            Some("plus"),
            "btn-square btn-outline btn-sm",
        )
        .into_string();
        assert!(html.contains("target_input=form_id"), "{html}");
        assert!(
            html.contains(
                r#"hx-get="/dashboard/forms/create/?name=p_forms.FormCreateForm&amp;target_input=form_id""#
            ),
            "{html}"
        );
        assert!(html.contains("name=p_forms.FormCreateForm"), "{html}");
        assert_eq!(
            html.matches("name=p_forms.FormCreateForm").count(),
            1,
            "{html}"
        );
    }
}
