//! HR registers no roles. Employment status lives on HR records.

use lariv_plugin_users::role_registry::{RoleRegistrar, RoleRegistry};

/// Names of the HR roles, in registration order.
pub const ALL: &[&str] = &[];

/// Registers the HR roles.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl RoleRegistrar for Hook {
    fn register_roles(self, registry: RoleRegistry) -> RoleRegistry {
        registry
    }
}
