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

#[cfg(test)]
mod tests {
    use crate::picker::picker_create_button;
    use crate::plugins::forms::keys::FormCreateModalKey;

    #[test]
    fn form_picker_create_button_embeds_target_input() {
        let html = picker_create_button::<FormCreateModalKey>(
            "form_id",
            Some("plus"),
            "btn-square btn-outline btn-sm",
        )
        .into_string();
        assert!(html.contains("target_input=form_id"), "{html}");
        assert!(
            html.contains(
                r#"hx-get="/forms/create/?name=p_forms.FormCreateForm&amp;target_input=form_id""#
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
