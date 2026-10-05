//! Compile-time role catalog.
//!
//! Plugins register [`Role`] types through [`RoleRegistrar`]. Mount copies each
//! type's constants into [`RoleRegistry`]. Stored values and allowlists use
//! [`Role::NAME`]; the role input shows [`Role::TITLE`] and [`Role::DESCRIPTION`].

use std::marker::PhantomData;

use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::App,
    capability::{CapHookExt, Capability, HasCapTag},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

tokio::task_local! {
    static CURRENT_ROLE_REGISTRY: RoleRegistry;
}

/// Stored name of the built-in superuser role. Allowlists treat this name as always permitted.
pub const SUPERUSER_ROLE_NAME: &str = "superuser";

/// Capability tag for the role catalog.
pub struct RoleRegistryTag;

/// A role defined in code. `NAME` is the stored identifier.
pub trait Role: Copy + Send + Sync + 'static {
    const NAME: &'static str;
    const TITLE: &'static str;
    const DESCRIPTION: &'static str;
}

/// One registered role, copied from a [`Role`] type at mount.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleMeta {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

/// Mounted catalog of role metadata, keyed by [`Role::NAME`].
#[derive(Clone, Debug, Default)]
pub struct RoleRegistry {
    roles: Vec<RoleMeta>,
}

impl RoleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append `R`. Panics when `R::NAME` is already registered.
    pub fn register<R: Role>(mut self) -> Self {
        if self.roles.iter().any(|role| role.name == R::NAME) {
            panic!("duplicate role name: {}", R::NAME);
        }
        self.roles.push(RoleMeta {
            name: R::NAME,
            title: R::TITLE,
            description: R::DESCRIPTION,
        });
        self
    }

    pub fn get(&self, name: &str) -> Option<&RoleMeta> {
        self.roles.iter().find(|role| role.name == name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Title for a stored name. Unknown names stay as the raw identifier.
    pub fn title_of(&self, name: &str) -> String {
        self.get(name)
            .map(|role| role.title.to_string())
            .unwrap_or_else(|| name.to_string())
    }

    /// `(NAME, TITLE, DESCRIPTION)` rows for the role dropdown.
    pub fn choices(&self) -> Vec<(String, String, String)> {
        self.roles
            .iter()
            .map(|role| {
                (
                    role.name.to_string(),
                    role.title.to_string(),
                    role.description.to_string(),
                )
            })
            .collect()
    }

    /// Accept a stored name. An empty name is valid only when the field is optional.
    pub fn validate(&self, name: &str, required: bool) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            if required {
                return Err("Select a role.".into());
            }
            return Ok(());
        }
        if self.contains(name) {
            Ok(())
        } else {
            Err(format!("Unknown role: {name}"))
        }
    }
}

/// Plugin hook that folds role types into [`RoleRegistry`].
pub trait RoleRegistrar: Sized {
    fn register_roles(self, registry: RoleRegistry) -> RoleRegistry;
}

/// Builder-phase role catalog capability.
#[derive(Clone, Default)]
pub struct RoleRegistryCap<Hooks> {
    pub hooks: Hooks,
    pub items: RoleRegistry,
    _tag: PhantomData<fn() -> RoleRegistryTag>,
}

impl<Hooks> RoleRegistryCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: RoleRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> RoleRegistryCap<HCons<Tagged<HTag, H>, Hooks>> {
        RoleRegistryCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for RoleRegistryCap<Hooks> {
    type Tag = RoleRegistryTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for RoleRegistryCap<Hooks> {
    type Hooked = RoleRegistryCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the catalog (tail first = install order).
pub trait FoldRoleRegistryHooks {
    fn fold(self, registry: RoleRegistry) -> RoleRegistry;
}

impl FoldRoleRegistryHooks for HNil {
    fn fold(self, registry: RoleRegistry) -> RoleRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldRoleRegistryHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldRoleRegistryHooks,
    H: RoleRegistrar + Copy,
{
    fn fold(self, registry: RoleRegistry) -> RoleRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_roles(registry)
    }
}

impl<Hooks> Capability for RoleRegistryCap<Hooks>
where
    Hooks: FoldRoleRegistryHooks,
{
    type Value = RoleRegistry;
    type Output = Tagged<RoleRegistryTag, RoleRegistry>;
    type Hooks = Hooks;
    type Items = RoleRegistry;

    fn mount(self) -> Self::Output {
        Tagged::new(self.hooks.fold(self.items))
    }
}

/// Attach an empty role catalog (prefer `cap_attach` in install steps).
pub fn with_role_registry<L, Proof>(app: App<L>) -> App<HCons<RoleRegistryCap<HNil>, L>>
where
    L: HList + CapTagAbsent<RoleRegistryTag, Proof>,
{
    app.add_capability(RoleRegistryCap::<HNil>::new())
}

/// Catalog scoped for this request, or empty when no registry was published.
pub fn current_role_registry() -> RoleRegistry {
    CURRENT_ROLE_REGISTRY
        .try_with(|registry| registry.clone())
        .unwrap_or_default()
}

/// Run `fut` with `registry` visible to [`current_role_registry`].
pub async fn scope<F>(registry: RoleRegistry, fut: F) -> F::Output
where
    F: std::future::Future,
{
    CURRENT_ROLE_REGISTRY.scope(registry, fut).await
}

/// Run `f` with `registry` visible to [`current_role_registry`].
pub fn scope_sync<R>(registry: RoleRegistry, f: impl FnOnce() -> R) -> R {
    CURRENT_ROLE_REGISTRY.sync_scope(registry, f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Sample;

    impl Role for Sample {
        const NAME: &'static str = "sample";
        const TITLE: &'static str = "Sample";
        const DESCRIPTION: &'static str = "A sample role.";
    }

    #[derive(Clone, Copy)]
    struct Other;

    impl Role for Other {
        const NAME: &'static str = "sample";
        const TITLE: &'static str = "Other";
        const DESCRIPTION: &'static str = "Duplicate name.";
    }

    #[test]
    fn choices_use_name_title_and_description() {
        let registry = RoleRegistry::new().register::<Sample>();
        assert_eq!(
            registry.choices(),
            vec![("sample".into(), "Sample".into(), "A sample role.".into())]
        );
        assert_eq!(registry.title_of("sample"), "Sample");
        assert_eq!(registry.title_of("missing"), "missing");
    }

    #[test]
    fn unknown_name_is_rejected_and_empty_is_optional() {
        let registry = RoleRegistry::new().register::<Sample>();
        assert!(registry.validate("sample", true).is_ok());
        assert_eq!(
            registry.validate("nope", true).unwrap_err(),
            "Unknown role: nope"
        );
        assert!(registry.validate("", false).is_ok());
        assert!(registry.validate("", true).is_err());
    }

    #[test]
    #[should_panic(expected = "duplicate role name: sample")]
    fn duplicate_role_name_panics() {
        let _registry = RoleRegistry::new().register::<Sample>().register::<Other>();
    }
}
