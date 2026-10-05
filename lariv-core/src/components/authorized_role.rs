//! Render markup only when the request principal may see it.
//!
//! The users plugin registers the check. Superuser always sees the markup.
//! With no scoped principal, nothing is rendered.

use maud::Markup;

/// Whether the current principal may see `roles` (superuser always may).
pub fn role_permitted(roles: &[String]) -> bool {
    crate::auth_hooks::role_permitted(roles)
}

/// Render `body` when the current user is a superuser or their role is in `roles`.
pub fn authorized_role(roles: &[String], body: Markup) -> Markup {
    if role_permitted(roles) {
        body
    } else {
        Markup::default()
    }
}
