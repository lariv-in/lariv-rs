//! Built-in roles registered by the users plugin.

use super::role_registry::{Role, RoleRegistrar, RoleRegistry};

macro_rules! role_ty {
    ($(#[$meta:meta])* $name:ident, $stored:literal, $title:literal, $desc:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $name;

        impl $name {
            pub const NAME: &'static str = $stored;
            pub const TITLE: &'static str = $title;
            pub const DESCRIPTION: &'static str = $desc;
        }

        impl Role for $name {
            const NAME: &'static str = $stored;
            const TITLE: &'static str = $title;
            const DESCRIPTION: &'static str = $desc;
        }
    };
}

role_ty!(
    /// Full access. Allowlists treat this role as always permitted.
    Superuser,
    "superuser",
    "Superuser",
    "Full access to every app and administration tool."
);

impl Superuser {
    pub fn matches(role: &str) -> bool {
        role == Self::NAME
    }
}

role_ty!(
    /// Default role for a user who has not been given a specific one.
    Unassigned,
    "unassigned",
    "Unassigned",
    "Default role for users who have not been assigned a specific role."
);

role_ty!(
    /// Operator role used by shared administrative tools.
    Admin,
    "admin",
    "Admin",
    "Administrator with access to shared operational tools."
);

/// Registers [`Superuser`], [`Unassigned`], and [`Admin`].
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl RoleRegistrar for Hook {
    fn register_roles(self, registry: RoleRegistry) -> RoleRegistry {
        registry
            .register::<Superuser>()
            .register::<Unassigned>()
            .register::<Admin>()
    }
}
