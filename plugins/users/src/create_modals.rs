//! Typed [`CreateModal`] / [`PickerModal`] wiring for user swap keys.

use super::keys::{UserCreateModalKey, UserSelectModalKey, UserSelectTableKey};
use super::routes::{UsersCreateGetRouteTag, UsersCreatePostRouteTag};

lariv_core::impl_create_modal!(
    UserCreateModalKey,
    UsersCreateGetRouteTag,
    UsersCreatePostRouteTag,
    "p_users.UserCreateForm"
);
lariv_core::impl_picker_modal!(UserSelectModalKey, UserSelectTableKey);

#[cfg(test)]
mod tests {
    use lariv_core::picker::picker_create_button;
    use crate::keys::UserCreateModalKey;

    #[test]
    fn user_picker_create_button_embeds_target_input() {
        let html = picker_create_button::<UserCreateModalKey>(
            "UserID",
            Some("plus"),
            "btn-square btn-outline btn-sm",
        )
        .into_string();
        assert!(html.contains("target_input=UserID"), "{html}");
        assert!(
            html.contains(
                r#"hx-get="/users/create/?name=p_users.UserCreateForm&amp;target_input=UserID""#
            ),
            "{html}"
        );
        assert!(html.contains("name=p_users.UserCreateForm"), "{html}");
    }
}
