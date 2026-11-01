//! Shared HR app sidebar — base links plus addon plugin patches at install time.
//!
//! The hub plugin registers this capability via `cap_attach` + `cap_hook(BaseHook)` in
//! [`define_plugin_install!`](lariv_core::plugin_install::define_plugin_install). HR addon
//! plugins add links with `cap_hook(HrSidebarTag, HrSidebarCap, Hook)`.
//!
//! Links are registered at mount time, but several decisions depend on the current request
//! (role gating, self-service vs. superuser URLs, manager-only queues). Those are evaluated at
//! render time through the optional [`HrSidebarLink::roles`], [`HrSidebarLink::url_resolver`]
//! and [`HrSidebarLink::visible`] function pointers.

use std::marker::PhantomData;
use std::sync::OnceLock;

use frunk::{HCons, HNil, hlist::HList};
use lariv_core::app::App;
use lariv_core::capability::{CapHookExt, Capability, HasCapTag};
use lariv_core::components::{
    SidebarMenu, SidebarMenuItem, SidebarNavLink, active_nav_key, normalize_nav_path,
    role_permitted, sidebar_menu, sidebar_menu_item_pane, sidebar_nav_items_pane,
};
use lariv_core::http::RouteUrl;
use lariv_core::tag::Tagged;
use lariv_core::traits::add::{AddCapability, CapTagAbsent};
use lariv_plugin_users::role_authorization::roles_for;
use maud::{Markup, PreEscaped};

use crate::hr_employee_detail::{HrEmployeeDetailRegistry, store_hr_employee_detail_links};
use crate::hr_preferences_patch::{HrPreferencesRegistry, store_hr_preferences_addons};
use crate::routes::{
    EmployeeDetailRouteTag, ExEmployeeDetailRouteTag, HrLeavePrefsGetRouteTag, HrPeopleView,
    HrPrefsAdmin, PeopleHubRouteTag,
};

/// Directory of a route path, dropping a `{param}` segment and the slash before it.
#[expect(
    clippy::indexing_slicing,
    reason = "const fn cannot use `get`; indices are bounds-checked by the loop"
)]
const fn route_dir(path: &'static str) -> &'static str {
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if i > 0 && bytes[i - 1] == b'/' {
                return path.split_at(i - 1).0;
            }
            return path.split_at(i).0;
        }
        i += 1;
    }
    path
}

const PEOPLE_MATCH_PREFIXES: &[&str] = &[
    PeopleHubRouteTag::PATH,
    route_dir(EmployeeDetailRouteTag::PATH),
    route_dir(ExEmployeeDetailRouteTag::PATH),
];
const PREFERENCES_MATCH_PREFIXES: &[&str] = &[HrLeavePrefsGetRouteTag::PATH];

static LINKS: OnceLock<Vec<HrSidebarLink>> = OnceLock::new();

/// Capability tag for the HR sidebar registry.
pub struct HrSidebarTag;

/// One navigation entry in the HR app sidebar.
#[derive(Clone, Debug)]
pub struct HrSidebarLink {
    pub section: &'static str,
    pub label: &'static str,
    pub url: String,
    pub order: u16,
    pub icon: Option<&'static str>,
    /// Extra path prefixes that mark this link active. Empty ⇒ use the resolved URL only.
    pub match_prefixes: &'static [&'static str],
    /// Render-time URL; overrides [`Self::url`] when present.
    pub url_resolver: Option<fn() -> String>,
    /// Render-time visibility; `None` ⇒ always visible.
    pub visible: Option<fn() -> bool>,
    /// Render-time role allowlist (typically `roles_for::<ViewCap>`); `None` ⇒ not role gated.
    pub roles: Option<fn() -> Vec<String>>,
}

impl HrSidebarLink {
    pub fn with_prefixes(mut self, match_prefixes: &'static [&'static str]) -> Self {
        self.match_prefixes = match_prefixes;
        self
    }

    pub fn with_url_resolver(mut self, resolver: fn() -> String) -> Self {
        self.url_resolver = Some(resolver);
        self
    }

    pub fn with_visible(mut self, visible: fn() -> bool) -> Self {
        self.visible = Some(visible);
        self
    }

    pub fn with_roles(mut self, roles: fn() -> Vec<String>) -> Self {
        self.roles = Some(roles);
        self
    }

    fn resolved_url(&self) -> String {
        match self.url_resolver {
            Some(resolve) => resolve(),
            None => self.url.clone(),
        }
    }

    /// Visibility and role gating for the current request.
    fn allowed(&self) -> bool {
        if self.visible.is_some_and(|visible| !visible()) {
            return false;
        }
        match self.roles {
            Some(roles) => role_permitted(&roles()),
            None => true,
        }
    }
}

/// Build a sidebar link from a typed route tag (compile-time path checking).
pub fn link<R: RouteUrl + Default>(
    section: &'static str,
    label: &'static str,
    order: u16,
    icon: Option<&'static str>,
) -> HrSidebarLink {
    HrSidebarLink {
        section,
        label,
        url: R::default().url(),
        order,
        icon,
        match_prefixes: &[],
        url_resolver: None,
        visible: None,
        roles: None,
    }
}

/// Like [`link`], but with extra path prefixes for active-state matching.
pub fn link_with_prefixes<R: RouteUrl + Default>(
    section: &'static str,
    label: &'static str,
    order: u16,
    icon: Option<&'static str>,
    match_prefixes: &'static [&'static str],
) -> HrSidebarLink {
    link::<R>(section, label, order, icon).with_prefixes(match_prefixes)
}

/// Strip query string and trailing slash (except root) for sidebar matching.
pub fn normalize_current_path(path_and_query: &str) -> String {
    normalize_nav_path(path_and_query)
}

/// A link after render-time gating and URL resolution.
struct ResolvedLink {
    section: &'static str,
    label: &'static str,
    url: String,
    icon: Option<&'static str>,
    match_prefixes: &'static [&'static str],
}

fn nav_links_from(links: &[ResolvedLink]) -> Vec<SidebarNavLink<'_>> {
    links
        .iter()
        .map(|link| SidebarNavLink {
            key: link.section,
            title: link.label,
            url: link.url.as_str(),
            icon_name: link.icon,
            match_prefixes: link.match_prefixes,
        })
        .collect()
}

/// Longest matching prefix wins; returns the active section key.
pub fn active_section_for_path<'a>(
    links: &'a [HrSidebarLink],
    current_path: &str,
) -> Option<&'a str> {
    let resolved: Vec<SidebarNavLink<'_>> = links
        .iter()
        .map(|link| SidebarNavLink {
            key: link.section,
            title: link.label,
            url: link.url.as_str(),
            icon_name: link.icon,
            match_prefixes: link.match_prefixes,
        })
        .collect();
    active_nav_key(&resolved, current_path)
}

/// Plugin hook for patching HR sidebar links, preferences and employee detail links.
pub trait HrSidebarRegistrar: Sized {
    fn register_hr_sidebar(self, cap: HrSidebarRegistry) -> HrSidebarRegistry;

    fn register_hr_preferences(self, cap: HrPreferencesRegistry) -> HrPreferencesRegistry {
        let _ = self;
        cap
    }

    fn register_employee_detail_links(
        self,
        cap: HrEmployeeDetailRegistry,
    ) -> HrEmployeeDetailRegistry {
        let _ = self;
        cap
    }
}

/// Sidebar link registry folded from base + addon hooks.
#[derive(Clone, Debug, Default)]
pub struct HrSidebarRegistry {
    links: Vec<HrSidebarLink>,
}

impl HrSidebarRegistry {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// Add a link; a duplicate `section` is ignored.
    pub fn push(mut self, link: HrSidebarLink) -> Self {
        if !self.links.iter().any(|l| l.section == link.section) {
            self.links.push(link);
        }
        self
    }

    fn sorted_links(self) -> Vec<HrSidebarLink> {
        let mut links = self.links;
        links.sort_by_key(|l| (l.order, l.label));
        links
    }

    /// Registered links in registration order (for tests and inspection).
    pub fn links(&self) -> &[HrSidebarLink] {
        &self.links
    }
}

/// Builder-phase HR sidebar capability.
#[derive(Clone, Default)]
pub struct HrSidebarCap<Hooks> {
    pub hooks: Hooks,
    pub items: HrSidebarRegistry,
    pub preferences: HrPreferencesRegistry,
    pub employee_detail: HrEmployeeDetailRegistry,
    _tag: PhantomData<fn() -> HrSidebarTag>,
}

impl<Hooks> HrSidebarCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: HrSidebarRegistry::new(),
            preferences: HrPreferencesRegistry::new(),
            employee_detail: HrEmployeeDetailRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> HrSidebarCap<HCons<Tagged<HTag, H>, Hooks>> {
        HrSidebarCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            preferences: self.preferences,
            employee_detail: self.employee_detail,
            _tag: PhantomData,
        }
    }

    /// Eagerly fold registrar hooks into items (testing / pre-mount inspection).
    pub fn resolve_hooks(self) -> HrSidebarCap<HNil>
    where
        Hooks: FoldHrRegistrarHooks,
    {
        let (items, preferences, employee_detail) =
            self.hooks
                .fold(self.items, self.preferences, self.employee_detail);
        HrSidebarCap {
            hooks: HNil,
            items,
            preferences,
            employee_detail,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for HrSidebarCap<Hooks> {
    type Tag = HrSidebarTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for HrSidebarCap<Hooks> {
    type Hooked = HrSidebarCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the HR registries (tail first = install order).
pub trait FoldHrRegistrarHooks {
    fn fold(
        self,
        sidebar: HrSidebarRegistry,
        preferences: HrPreferencesRegistry,
        employee_detail: HrEmployeeDetailRegistry,
    ) -> (
        HrSidebarRegistry,
        HrPreferencesRegistry,
        HrEmployeeDetailRegistry,
    );
}

impl FoldHrRegistrarHooks for HNil {
    fn fold(
        self,
        sidebar: HrSidebarRegistry,
        preferences: HrPreferencesRegistry,
        employee_detail: HrEmployeeDetailRegistry,
    ) -> (
        HrSidebarRegistry,
        HrPreferencesRegistry,
        HrEmployeeDetailRegistry,
    ) {
        (sidebar, preferences, employee_detail)
    }
}

impl<Plugin, H, Tail> FoldHrRegistrarHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldHrRegistrarHooks,
    H: HrSidebarRegistrar + Copy,
{
    fn fold(
        self,
        sidebar: HrSidebarRegistry,
        preferences: HrPreferencesRegistry,
        employee_detail: HrEmployeeDetailRegistry,
    ) -> (
        HrSidebarRegistry,
        HrPreferencesRegistry,
        HrEmployeeDetailRegistry,
    ) {
        let (sidebar, preferences, employee_detail) =
            self.tail.fold(sidebar, preferences, employee_detail);
        let hook = self.head.value;
        (
            hook.register_hr_sidebar(sidebar),
            hook.register_hr_preferences(preferences),
            hook.register_employee_detail_links(employee_detail),
        )
    }
}

impl<Hooks> Capability for HrSidebarCap<Hooks>
where
    Hooks: FoldHrRegistrarHooks,
{
    type Value = HrSidebarRegistry;
    type Output = Tagged<HrSidebarTag, HrSidebarRegistry>;
    type Hooks = Hooks;
    type Items = HrSidebarRegistry;

    fn mount(self) -> Self::Output {
        let (registry, preferences, employee_detail) =
            self.hooks
                .fold(self.items, self.preferences, self.employee_detail);
        let sorted = registry.clone().sorted_links();
        if LINKS.set(sorted).is_err() {
            tracing::error!("hr sidebar LINKS already initialized");
        }
        store_hr_preferences_addons(&preferences);
        store_hr_employee_detail_links(&employee_detail);
        Tagged::new(registry)
    }
}

/// Attach an empty HR sidebar capability to the app builder.
///
/// Prefer `cap_attach` in the hub plugin install steps; this helper remains for
/// manual wiring or tests outside the install macro.
pub fn with_hr_sidebar<L, Proof>(app: App<L>) -> App<HCons<HrSidebarCap<HNil>, L>>
where
    L: HList + CapTagAbsent<HrSidebarTag, Proof>,
{
    app.add_capability(HrSidebarCap::new())
}

/// Registered links, sorted by order.
///
/// Before app mount (unit tests, pre-mount rendering) this falls back to the base hook's links.
fn registered_links() -> Vec<HrSidebarLink> {
    match LINKS.get() {
        Some(links) => links.clone(),
        None => BaseHook
            .register_hr_sidebar(HrSidebarRegistry::new())
            .sorted_links(),
    }
}

/// Links the current request may see, with URLs resolved.
fn visible_links() -> Vec<ResolvedLink> {
    registered_links()
        .into_iter()
        .filter(HrSidebarLink::allowed)
        .map(|link| ResolvedLink {
            url: link.resolved_url(),
            section: link.section,
            label: link.label,
            icon: link.icon,
            match_prefixes: link.match_prefixes,
        })
        .collect()
}

/// URL of the first sidebar link the current request may see (lowest order).
pub fn first_visible_url() -> Option<String> {
    visible_links().into_iter().next().map(|link| link.url)
}

fn wrap_menu(children: Markup) -> Option<Markup> {
    let raw = children.into_string();
    if raw.trim().is_empty() {
        None
    } else {
        Some(sidebar_menu(SidebarMenu {
            title: "HR",
            children: PreEscaped(raw),
        }))
    }
}

/// Render the patched HR sidebar, highlighting the link that matches `current_path`.
///
/// `None` when the current user may not see any link.
pub fn hr_sidebar(current_path: &str) -> Option<Markup> {
    let links = visible_links();
    let nav = nav_links_from(&links);
    wrap_menu(sidebar_nav_items_pane(&nav, current_path))
}

/// Render the patched HR sidebar, highlighting the link whose section key equals `active`.
///
/// `None` when the current user may not see any link.
pub fn hr_menu_for_active(active: &str) -> Option<Markup> {
    let mut children = Markup::default();
    for link in visible_links() {
        let item = sidebar_menu_item_pane(SidebarMenuItem {
            title: link.label,
            url: &link.url,
            icon_name: link.icon,
            active: link.section == active,
        });
        children = maud::html! { (children) (item) };
    }
    wrap_menu(children)
}

/// True when the signed-in user is not a superuser (self-service addon pages).
pub fn self_service() -> bool {
    use lariv_plugin_users::role_authorization::current_auth;
    use lariv_plugin_users::roles::Superuser;
    current_auth().is_some_and(|auth| !Superuser::matches(&auth.role))
}

fn not_self_service() -> bool {
    !self_service()
}

#[derive(Clone, Copy, Default)]
pub struct BaseHook;

impl HrSidebarRegistrar for BaseHook {
    fn register_hr_sidebar(self, cap: HrSidebarRegistry) -> HrSidebarRegistry {
        cap.push(
            link_with_prefixes::<PeopleHubRouteTag>(
                "people",
                "People",
                10,
                None,
                PEOPLE_MATCH_PREFIXES,
            )
            .with_roles(roles_for::<HrPeopleView>),
        )
        .push(
            link_with_prefixes::<HrLeavePrefsGetRouteTag>(
                "preferences",
                "Preferences",
                1000,
                None,
                PREFERENCES_MATCH_PREFIXES,
            )
            .with_visible(not_self_service)
            .with_roles(roles_for::<HrPrefsAdmin>),
        )
    }
}
