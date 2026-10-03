//! Render markup only when the request principal may see it.
//!
//! The role vec is the patched allowlist (see [`crate::plugins::users::role_authorization::roles_for`]).
//! Superuser always sees the markup. With no scoped principal, nothing is rendered.

use maud::Markup;

use crate::plugins::users::role_authorization::{current_auth, principal_allowed};

/// Whether the current principal may see `roles` (superuser always may).
pub fn role_permitted(roles: &[String]) -> bool {
    current_auth().is_some_and(|auth| principal_allowed(&auth, roles))
}

/// Render `body` when the current user is a superuser or their role is in `roles`.
pub fn authorized_role(roles: &[String], body: Markup) -> Markup {
    if role_permitted(roles) {
        body
    } else {
        Markup::default()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use maud::html;

    use super::*;
    use crate::plugins::users::{
        entities::user::Model as User,
        role_authorization::{RoleAuthorizationRegistry, with_principal},
        state::AuthContext,
    };

    fn auth(is_superuser: bool, role: &str) -> AuthContext {
        AuthContext {
            user: User {
                id: 1,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                name: "Ada".into(),
                email: "ada@example.com".into(),
                phone: "1".into(),
                is_superuser,
                role_id: 1,
                password_hash: Some(vec![]),
                password_salt: Some(vec![]),
                timezone: "UTC".into(),
            },
            role: role.into(),
            timezone: "UTC".into(),
        }
    }

    #[test]
    fn superuser_sees_body_when_allowlist_is_empty() {
        let markup = with_principal(
            auth(true, "employee"),
            RoleAuthorizationRegistry::new(),
            || authorized_role(&[], html! { span { "edit" } }),
        );
        assert!(markup.into_string().contains("edit"));
    }

    #[test]
    fn other_role_is_hidden_when_allowlist_is_empty() {
        let markup = with_principal(
            auth(false, "employee"),
            RoleAuthorizationRegistry::new(),
            || authorized_role(&[], html! { span { "edit" } }),
        );
        assert!(markup.into_string().is_empty());
    }

    #[test]
    fn matching_role_sees_body() {
        let markup = with_principal(
            auth(false, "employee"),
            RoleAuthorizationRegistry::new(),
            || authorized_role(&["employee".into()], html! { span { "edit" } }),
        );
        assert!(markup.into_string().contains("edit"));
    }

    #[test]
    fn no_principal_hides_body() {
        let markup = authorized_role(&["employee".into()], html! { span { "edit" } });
        assert!(markup.into_string().is_empty());
    }
}
