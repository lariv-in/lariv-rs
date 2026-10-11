//! Install-time registry of service-worker modules.
//!
//! Each entry is a [`RouteTag`](lariv_core::http::RouteTag) whose `PATH` is under `/sw/`
//! and the JS source that path serves. [`/serviceworker.js`](crate::handlers::service_worker)
//! imports those paths as an ES module. Plugins add modules with
//! [`ServiceWorkerRegistrar`] via `cap_hook`.

use std::marker::PhantomData;

use frunk::{HCons, HNil};
use lariv_core::capability::{CapHookExt, Capability, HasCapTag};
use lariv_core::http::RouteTag;
use lariv_core::tag::Tagged;

use crate::routes::PwaDefaultServiceWorkerRouteTag;

/// Capability tag for the service-worker module registry.
pub struct ServiceWorkerTag;

/// JS module registered under a typed route tag. The URL is [`RouteTag::PATH`].
pub struct ServiceWorkerModule<Tag> {
    source: &'static str,
    _tag: PhantomData<fn() -> Tag>,
}

impl<Tag> ServiceWorkerModule<Tag> {
    /// JS source served at `Tag::PATH`.
    pub const fn new(source: &'static str) -> Self {
        Self {
            source,
            _tag: PhantomData,
        }
    }
}

/// One module after the route tag has been erased to its path.
#[derive(Clone, Copy, Debug)]
pub struct RegisteredServiceWorker {
    path: &'static str,
    source: &'static str,
}

impl RegisteredServiceWorker {
    /// URL imported by `/serviceworker.js` and served by the module route.
    pub fn path(&self) -> &'static str {
        self.path
    }

    /// Module source for [`Self::path`].
    pub fn source(&self) -> &'static str {
        self.source
    }
}

/// Folded list of service-worker modules (mounted capability value).
#[derive(Clone, Debug, Default)]
pub struct ServiceWorkerRegistry {
    modules: Vec<RegisteredServiceWorker>,
}

impl ServiceWorkerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `module`. A duplicate path or a path outside `/sw/*.js` is left unchanged.
    ///
    /// Install order wins: the first registration of a path is kept.
    pub fn register<Tag: RouteTag>(mut self, module: ServiceWorkerModule<Tag>) -> Self {
        let path = Tag::PATH;
        if !is_module_path(path) {
            tracing::warn!(
                path,
                "p_pwa: service worker module path must start with /sw/ and end with .js; skipping"
            );
            return self;
        }
        if self
            .modules
            .iter()
            .any(|registered| registered.path == path)
        {
            return self;
        }
        self.modules.push(RegisteredServiceWorker {
            path,
            source: module.source,
        });
        self
    }

    /// Source registered at `path`.
    pub fn get(&self, path: &str) -> Option<&'static str> {
        self.modules
            .iter()
            .find(|registered| registered.path == path)
            .map(RegisteredServiceWorker::source)
    }

    /// Modules in registration order.
    pub fn modules(&self) -> &[RegisteredServiceWorker] {
        &self.modules
    }

    /// ES module body that imports every registered path.
    pub fn entry_script(&self) -> String {
        let mut body = String::new();
        for module in &self.modules {
            body.push_str("import \"");
            body.push_str(module.path);
            body.push_str("\";\n");
        }
        body
    }
}

fn is_module_path(path: &str) -> bool {
    path.starts_with("/sw/") && path.ends_with(".js") && path.len() > "/sw/.js".len()
}

/// Plugin hook for registering service-worker modules at install time.
pub trait ServiceWorkerRegistrar: Sized {
    fn register_service_workers(self, registry: ServiceWorkerRegistry) -> ServiceWorkerRegistry;
}

/// Builder-phase service-worker capability.
#[derive(Clone, Default)]
pub struct ServiceWorkerCap<Hooks> {
    pub hooks: Hooks,
    pub items: ServiceWorkerRegistry,
    _tag: PhantomData<fn() -> ServiceWorkerTag>,
}

impl<Hooks> ServiceWorkerCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: ServiceWorkerRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> ServiceWorkerCap<HCons<Tagged<HTag, H>, Hooks>> {
        ServiceWorkerCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for ServiceWorkerCap<Hooks> {
    type Tag = ServiceWorkerTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for ServiceWorkerCap<Hooks> {
    type Hooked = ServiceWorkerCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the registry (tail first = install order).
pub trait FoldServiceWorkerRegistrarHooks {
    fn fold(self, registry: ServiceWorkerRegistry) -> ServiceWorkerRegistry;
}

impl FoldServiceWorkerRegistrarHooks for HNil {
    fn fold(self, registry: ServiceWorkerRegistry) -> ServiceWorkerRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldServiceWorkerRegistrarHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldServiceWorkerRegistrarHooks,
    H: ServiceWorkerRegistrar + Copy,
{
    fn fold(self, registry: ServiceWorkerRegistry) -> ServiceWorkerRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_service_workers(registry)
    }
}

impl<Hooks> Capability for ServiceWorkerCap<Hooks>
where
    Hooks: FoldServiceWorkerRegistrarHooks,
{
    type Value = ServiceWorkerRegistry;
    type Output = Tagged<ServiceWorkerTag, ServiceWorkerRegistry>;
    type Hooks = Hooks;
    type Items = ServiceWorkerRegistry;

    fn mount(self) -> Self::Output {
        let registry = self.hooks.fold(self.items);
        Tagged::new(registry)
    }
}

/// Registers the default cache/offline module at [`PwaDefaultServiceWorkerRouteTag`].
#[derive(Clone, Copy, Default)]
pub struct BaseHook;

impl ServiceWorkerRegistrar for BaseHook {
    fn register_service_workers(self, registry: ServiceWorkerRegistry) -> ServiceWorkerRegistry {
        registry.register(ServiceWorkerModule::<PwaDefaultServiceWorkerRouteTag>::new(
            include_str!("default_sw.js"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::PwaOfflineRouteTag;

    struct ExtraSwRouteTag;
    struct DuplicateSwRouteTag;
    struct OutsideSwRouteTag;
    struct BareJsRouteTag;

    impl RouteTag for ExtraSwRouteTag {
        const PATH: &'static str = "/sw/extra.js";
    }

    impl RouteTag for DuplicateSwRouteTag {
        const PATH: &'static str = "/sw/pwa.js";
    }

    impl RouteTag for OutsideSwRouteTag {
        const PATH: &'static str = "/other/extra.js";
    }

    impl RouteTag for BareJsRouteTag {
        const PATH: &'static str = "/sw/.js";
    }

    #[test]
    fn base_hook_registers_default_module() {
        let registry = BaseHook.register_service_workers(ServiceWorkerRegistry::new());
        let source = registry
            .get(PwaDefaultServiceWorkerRouteTag::PATH)
            .expect("default module");
        assert!(source.contains(PwaOfflineRouteTag::PATH));
        assert!(source.contains("lariv-pwa-v1"));
        assert_eq!(
            registry.entry_script(),
            format!("import \"{}\";\n", PwaDefaultServiceWorkerRouteTag::PATH)
        );
    }

    #[test]
    fn duplicate_path_keeps_the_first_registration() {
        let registry = ServiceWorkerRegistry::new()
            .register(ServiceWorkerModule::<PwaDefaultServiceWorkerRouteTag>::new(
                "first",
            ))
            .register(ServiceWorkerModule::<DuplicateSwRouteTag>::new("second"));
        assert_eq!(
            registry.get(PwaDefaultServiceWorkerRouteTag::PATH),
            Some("first")
        );
        assert_eq!(registry.modules().len(), 1);
    }

    #[test]
    fn later_module_is_imported_after_the_first() {
        let registry = ServiceWorkerRegistry::new()
            .register(ServiceWorkerModule::<PwaDefaultServiceWorkerRouteTag>::new(
                "default",
            ))
            .register(ServiceWorkerModule::<ExtraSwRouteTag>::new("extra"));
        assert_eq!(
            registry.entry_script(),
            "import \"/sw/pwa.js\";\nimport \"/sw/extra.js\";\n"
        );
        assert_eq!(registry.get(ExtraSwRouteTag::PATH), Some("extra"));
    }

    #[test]
    fn paths_outside_sw_prefix_are_skipped() {
        let registry = ServiceWorkerRegistry::new()
            .register(ServiceWorkerModule::<OutsideSwRouteTag>::new("nope"))
            .register(ServiceWorkerModule::<BareJsRouteTag>::new("nope"));
        assert!(registry.modules().is_empty());
        assert!(registry.get(OutsideSwRouteTag::PATH).is_none());
    }
}
