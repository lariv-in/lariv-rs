//! Typed [`CreateModal`] wiring for user swap keys.

use lariv_core::web::CreateModal;

use crate::keys::USER_CREATE_MODAL;
use crate::routes::{UsersCreateGetRouteTag, UsersCreatePostRouteTag};

pub const USER_CREATE: CreateModal<UsersCreateGetRouteTag, UsersCreatePostRouteTag> = CreateModal {
    key: USER_CREATE_MODAL,
    form_name: "p_users.UserCreateForm",
    get: UsersCreateGetRouteTag,
    post: UsersCreatePostRouteTag,
};

#[cfg(test)]
mod tests {
    use super::USER_CREATE;
    use lariv_core::picker::picker_create_button;

    #[test]
    fn user_picker_create_button_embeds_target_input() {
        let html = picker_create_button(
            USER_CREATE,
            "UserID",
            Some("plus"),
            "btn-square btn-outline btn-sm",
        )
        .into_string();
        assert!(html.contains("target_input=UserID"), "{html}");
        assert!(
            html.contains(
                r#"hx-get="/dashboard/users/create/?name=p_users.UserCreateForm&amp;target_input=UserID""#
            ),
            "{html}"
        );
        assert!(html.contains("name=p_users.UserCreateForm"), "{html}");
    }
}
