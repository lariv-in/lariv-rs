//! App catalog capability — plugins register launchable tiles; dashboard reads them live.

use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, mount_with_hooks},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for the app catalog.
pub struct AppsTag;

/// Kind of registered plugin (mirrors Go `PluginType`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginType {
    /// Shows on the apps grid.
    App,
    /// Addon / infrastructure — hidden from the grid.
    Addon,
}

/// A launchable tile on the dashboard apps grid (Go `lariv.Plugin` app metadata).
#[derive(Clone, Debug)]
pub struct AppTile {
    pub key: String,
    pub verbose_name: String,
    pub href: String,
    /// Short label / icon name used as the tile icon.
    pub icon: String,
    pub plugin_type: PluginType,
    /// If non-empty, only these roles (or superuser) see the tile.
    pub roles: Vec<String>,
}

/// Plugin hook for appending app tiles onto an [`AppsCapability`].
pub trait AppsRegistrar: Sized {
    fn register_apps(self, apps: AppsCapability) -> AppsCapability;
}

/// Builder-phase apps capability.
#[derive(Clone, Debug, Default)]
pub struct AppsCapability {
    apps: Vec<AppTile>,
}

impl AppsCapability {
    pub fn new() -> Self {
        Self { apps: Vec::new() }
    }

    /// Register a tile (idempotent on `key`: replaces an existing entry).
    pub fn register(mut self, tile: AppTile) -> Self {
        if let Some(existing) = self.apps.iter_mut().find(|a| a.key == tile.key) {
            *existing = tile;
        } else {
            self.apps.push(tile);
        }
        self
    }

    pub fn apps(&self) -> &[AppTile] {
        &self.apps
    }

    /// Apps visible on the dashboard grid for the given role.
    pub fn visible_apps(&self, role: &str, is_superuser: bool) -> Vec<AppTile> {
        let mut apps: Vec<_> = self
            .apps
            .iter()
            .filter(|a| a.plugin_type == PluginType::App)
            .filter(|a| {
                if is_superuser || a.roles.is_empty() {
                    true
                } else {
                    a.roles.iter().any(|r| r == role)
                }
            })
            .cloned()
            .collect();
        apps.sort_by(|a, b| a.verbose_name.cmp(&b.verbose_name));
        apps
    }
}

/// Builder-phase apps capability.
pub type AppsCap<Hooks> = CapStore<AppsTag, Hooks, AppsCapability>;

impl<Hooks> AppsCap<Hooks> {
    pub fn resolve_hooks<Proof>(self) -> AppsCap<HNil>
    where
        Hooks: ApplyHooks<AppsCapability, Proof, Output = AppsCapability>,
    {
        CapStore::with_items(self.hooks.apply_hooks(self.items))
    }
}

impl<Plugin, H, Tail, TailProof> ApplyHooks<AppsCapability, (TailProof, ())>
    for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: ApplyHooks<AppsCapability, TailProof, Output = AppsCapability>,
    H: AppsRegistrar,
{
    type Output = AppsCapability;

    fn apply_hooks(self, items: AppsCapability) -> Self::Output {
        let items = self.tail.apply_hooks(items);
        self.head.value.register_apps(items)
    }
}

impl<Hooks> Capability for AppsCap<Hooks>
where
    Hooks: ApplyHooks<AppsCapability, (), Output = AppsCapability>,
{
    type Value = AppsCapability;
    type Output = Tagged<AppsTag, AppsCapability>;
    type Hooks = Hooks;
    type Items = AppsCapability;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, |items| items)
    }
}

/// Register a dashboard app tile for a plugin.
///
/// ```ignore
/// define_register_apps! {
///     plugin: UsersTag;
///     key: "p_users";
///     name: "Users";
///     href: "/users";
///     icon: "users";
///     roles: [];
/// }
/// ```
#[macro_export]
macro_rules! define_register_apps {
    (
        plugin: $plugin:ty;
        key: $key:expr;
        name: $name:expr;
        href: $href:expr;
        icon: $icon:expr;
        $(plugin_type: $ptype:expr;)?
        roles: [$($role:expr),* $(,)?];
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct Hook;

        impl $crate::apps::AppsRegistrar for Hook {
            fn register_apps(self, apps: $crate::apps::AppsCapability) -> $crate::apps::AppsCapability {
                apps.register($crate::apps::AppTile {
                    key: ::std::convert::Into::into($key),
                    verbose_name: ::std::convert::Into::into($name),
                    href: ::std::convert::Into::into($href),
                    icon: ::std::convert::Into::into($icon),
                    plugin_type: $crate::define_register_apps!(@plugin_type $($ptype)?),
                    roles: vec![$(::std::convert::Into::into($role)),*],
                })
            }
        }
    };
    (@plugin_type) => {
        $crate::apps::PluginType::App
    };
    (@plugin_type $ptype:expr) => {
        $ptype
    };
}

pub use crate::define_register_apps;

pub fn with_apps<L, Proof>(app: App<L>) -> App<HCons<AppsCap<HNil>, L>>
where
    L: HList + CapTagAbsent<AppsTag, Proof>,
{
    app.add_capability(CapStore::with_items(AppsCapability::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::blog::BlogTag;
    use crate::plugins::users::UsersTag;

    #[test]
    fn visible_after_register_apps() {
        let apps = AppsCapability::new();
        let apps = crate::plugins::users::apps::Hook.register_apps(apps);
        let apps = crate::plugins::blog::apps::Hook.register_apps(apps);
        assert_eq!(apps.apps().len(), 2);
        let visible = apps.visible_apps("admin", false);
        let keys: Vec<_> = visible.iter().map(|t| t.key.as_str()).collect();
        assert!(keys.contains(&"p_users"), "{keys:?}");
        assert!(keys.contains(&"p_blog"), "{keys:?}");
    }
}
