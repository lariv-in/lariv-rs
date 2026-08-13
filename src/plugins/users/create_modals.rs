//! Typed [`CreateModal`] / [`PickerModal`] wiring for user and role swap keys.

use super::keys::{
    RoleCreateModalKey, RoleSelectModalKey, RoleSelectTableKey, UserCreateModalKey,
    UserSelectModalKey, UserSelectTableKey,
};
use super::routes::{
    UsersCreateGetRouteTag, UsersCreatePostRouteTag, UsersRolesCreateGetRouteTag,
    UsersRolesCreatePostRouteTag,
};

crate::impl_create_modal!(
    UserCreateModalKey,
    UsersCreateGetRouteTag,
    UsersCreatePostRouteTag,
    "p_users.UserCreateForm"
);
crate::impl_create_modal!(
    RoleCreateModalKey,
    UsersRolesCreateGetRouteTag,
    UsersRolesCreatePostRouteTag,
    "p_users.RoleCreateForm"
);
crate::impl_picker_modal!(UserSelectModalKey, UserSelectTableKey);
crate::impl_picker_modal!(RoleSelectModalKey, RoleSelectTableKey);

#[cfg(test)]
mod tests {
    use crate::picker::picker_create_button;
    use crate::plugins::users::keys::{RoleCreateModalKey, UserCreateModalKey};

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

    #[test]
    fn role_picker_create_button_embeds_target_input() {
        let html = picker_create_button::<RoleCreateModalKey>(
            "RoleID",
            Some("plus"),
            "btn-square btn-outline btn-sm",
        )
        .into_string();
        assert!(html.contains("target_input=RoleID"), "{html}");
        assert!(
            html.contains(
                r#"hx-get="/users/roles/create/?name=p_users.RoleCreateForm&amp;target_input=RoleID""#
            ),
            "{html}"
        );
        assert!(html.contains("name=p_users.RoleCreateForm"), "{html}");
    }
}
