//! Typed [`CreateModal`] wiring for filesystem swap keys.

use lariv_core::web::CreateModal;

use crate::keys::V_NODE_CREATE_MODAL;
use crate::routes::{VNodeCreateGetRouteTag, VNodeCreatePostRouteTag};

pub const V_NODE_CREATE: CreateModal<VNodeCreateGetRouteTag, VNodeCreatePostRouteTag> =
    CreateModal {
        key: V_NODE_CREATE_MODAL,
        form_name: "p_filesystem.VNodeCreateForm",
        get: VNodeCreateGetRouteTag,
        post: VNodeCreatePostRouteTag,
    };

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_core::components::picker::picker_create_button;
    use lariv_core::web::{CreateModal, modal_create_href_for_picker};

    #[test]
    fn vnode_picker_create_button_embeds_target_input() {
        let html = picker_create_button(
            V_NODE_CREATE,
            "ParentID",
            Some("plus"),
            "btn-square btn-outline btn-sm",
        )
        .into_string();
        assert!(html.contains("target_input=ParentID"), "{html}");
        assert!(
            html.contains(
                r#"hx-get="/dashboard/filesystem/create/?name=p_filesystem.VNodeCreateForm&amp;target_input=ParentID""#
            ),
            "{html}"
        );
        assert!(html.contains("name=p_filesystem.VNodeCreateForm"), "{html}");
    }

    #[test]
    fn vnode_picker_create_href_keeps_parent_route() {
        let href = modal_create_href_for_picker(
            "/filesystem/create/in/7/",
            V_NODE_CREATE.form_name,
            "ParentID",
        );
        assert!(href.contains("/filesystem/create/in/7/"), "{href}");
        assert!(href.contains("name=p_filesystem.VNodeCreateForm"), "{href}");
        assert!(href.contains("target_input=ParentID"), "{href}");
    }
}
