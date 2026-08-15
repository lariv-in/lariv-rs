//! Typed [`CreateModal`] / [`PickerModal`] wiring for filesystem swap keys.

use super::keys::{VNodeCreateModalKey, VNodeSelectModalKey, VNodeSelectTableKey};
use super::routes::{VNodeCreateGetRouteTag, VNodeCreatePostRouteTag};

crate::impl_create_modal!(
    VNodeCreateModalKey,
    VNodeCreateGetRouteTag,
    VNodeCreatePostRouteTag,
    "p_filesystem.VNodeCreateForm"
);
crate::impl_picker_modal!(VNodeSelectModalKey, VNodeSelectTableKey);

#[cfg(test)]
mod tests {
    use crate::picker::picker_create_button;
    use crate::plugins::filesystem::keys::VNodeCreateModalKey;
    use crate::web::{CreateModal, modal_create_href_for_picker};

    #[test]
    fn vnode_picker_create_button_embeds_target_input() {
        let html = picker_create_button::<VNodeCreateModalKey>(
            "ParentID",
            Some("plus"),
            "btn-square btn-outline btn-sm",
        )
        .into_string();
        assert!(html.contains("target_input=ParentID"), "{html}");
        assert!(
            html.contains(
                r#"hx-get="/filesystem/create/?name=p_filesystem.VNodeCreateForm&amp;target_input=ParentID""#
            ),
            "{html}"
        );
        assert!(html.contains("name=p_filesystem.VNodeCreateForm"), "{html}");
    }

    #[test]
    fn vnode_picker_create_href_keeps_parent_route() {
        let href = modal_create_href_for_picker(
            "/filesystem/create/in/7/",
            VNodeCreateModalKey::FORM_NAME,
            "ParentID",
        );
        assert!(href.contains("/filesystem/create/in/7/"), "{href}");
        assert!(href.contains("name=p_filesystem.VNodeCreateForm"), "{href}");
        assert!(href.contains("target_input=ParentID"), "{href}");
    }
}
