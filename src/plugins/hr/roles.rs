//! HR roles registered on the role catalog.

use crate::plugins::users::role_registry::{Role, RoleRegistrar, RoleRegistry};

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
    /// Person who has applied and is being considered for a position.
    Applicant,
    "applicant",
    "Applicant",
    "Person who has applied and is being considered for a position."
);

role_ty!(
    /// Employee serving a probationary period before confirmation.
    Probation,
    "probation",
    "Probation",
    "Employee serving a probationary period before confirmation."
);

role_ty!(
    /// Current employee of the organization.
    Employee,
    "employee",
    "Employee",
    "Current employee of the organization."
);

role_ty!(
    /// Former employee who has left the organization.
    ExEmployee,
    "ex-employee",
    "Ex-Employee",
    "Former employee who has left the organization."
);

/// Names of the HR roles, in registration order.
pub const ALL: &[&str] = &[
    Applicant::NAME,
    Probation::NAME,
    Employee::NAME,
    ExEmployee::NAME,
];

/// Registers the HR roles.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl RoleRegistrar for Hook {
    fn register_roles(self, registry: RoleRegistry) -> RoleRegistry {
        registry
            .register::<Applicant>()
            .register::<Probation>()
            .register::<Employee>()
            .register::<ExEmployee>()
    }
}
