//! Sections and detail-sidebar entries other finance plugins contribute for a customer.

use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use frunk::{HCons, HNil, hlist::HList};
use lariv_core::app::App;
use lariv_core::capability::{CapHookExt, Capability, HasCapTag};
use lariv_core::tag::Tagged;
use lariv_core::traits::add::{AddCapability, CapTagAbsent};
use maud::Markup;
use sea_orm::DatabaseConnection;

/// Capability tag for customer journal sections.
pub struct CustomerJournalTag;

/// Request data a journal section needs to render its tables.
pub struct CustomerJournalRender {
    pub customer_id: i64,
    pub db: DatabaseConnection,
    pub timezone: String,
    pub cookie_header: Option<String>,
}

/// One block on the customer journal page (invoices, payments, …).
#[async_trait]
pub trait CustomerJournalSection: Send + Sync {
    fn order(&self) -> u16;
    async fn render(&self, ctx: &CustomerJournalRender) -> Markup;
}

/// Folded list of journal sections.
#[derive(Clone, Default)]
pub struct CustomerJournalRegistry {
    sections: Vec<Arc<dyn CustomerJournalSection>>,
}

impl CustomerJournalRegistry {
    pub fn push(mut self, section: Arc<dyn CustomerJournalSection>) -> Self {
        self.sections.push(section);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    pub async fn render(&self, ctx: &CustomerJournalRender) -> Markup {
        let mut sections = self.sections.clone();
        sections.sort_by_key(|section| section.order());
        let mut body = Markup::default();
        for section in sections {
            let part = section.render(ctx).await;
            body = maud::html! { (body) (part) };
        }
        body
    }
}

/// One extra entry in the customer detail sidebar, after Detail and Journal.
#[derive(Clone, Debug)]
pub struct CustomerSidebarLink {
    /// Active-state key compared against the menu's `active` argument.
    pub key: &'static str,
    pub title: &'static str,
    /// Builds the link target for one customer id.
    pub url_fn: fn(customer_id: i64) -> String,
    pub order: u16,
}

/// Customer detail-sidebar links folded from addon hooks.
#[derive(Clone, Debug, Default)]
pub struct CustomerSidebarRegistry {
    links: Vec<CustomerSidebarLink>,
}

impl CustomerSidebarRegistry {
    pub fn push(mut self, link: CustomerSidebarLink) -> Self {
        if !self.links.iter().any(|existing| existing.key == link.key) {
            self.links.push(link);
        }
        self
    }

    fn sorted_links(self) -> Vec<CustomerSidebarLink> {
        let mut links = self.links;
        links.sort_by_key(|link| (link.order, link.title));
        links
    }
}

static SIDEBAR_LINKS: OnceLock<Vec<CustomerSidebarLink>> = OnceLock::new();

fn store_customer_sidebar_links(registry: &CustomerSidebarRegistry) {
    if SIDEBAR_LINKS.set(registry.clone().sorted_links()).is_err() {
        tracing::error!("customer sidebar links already initialized");
    }
}

/// Sidebar entries registered by invoices, payments, and other customer addons.
pub fn customer_sidebar_links() -> Vec<CustomerSidebarLink> {
    SIDEBAR_LINKS.get().cloned().unwrap_or_default()
}

/// Plugin hook that appends journal sections and customer sidebar entries.
pub trait CustomerJournalRegistrar: Sized {
    fn register_customer_journal(
        self,
        registry: CustomerJournalRegistry,
    ) -> CustomerJournalRegistry;

    fn register_customer_sidebar(
        self,
        registry: CustomerSidebarRegistry,
    ) -> CustomerSidebarRegistry {
        let _ = self;
        registry
    }
}

/// Builder-phase customer journal capability.
#[derive(Clone, Default)]
pub struct CustomerJournalCap<Hooks> {
    pub hooks: Hooks,
    pub items: CustomerJournalRegistry,
    pub sidebar: CustomerSidebarRegistry,
    _tag: PhantomData<fn() -> CustomerJournalTag>,
}

impl<Hooks> CustomerJournalCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: CustomerJournalRegistry::default(),
            sidebar: CustomerSidebarRegistry::default(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> CustomerJournalCap<HCons<Tagged<HTag, H>, Hooks>> {
        CustomerJournalCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            sidebar: self.sidebar,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for CustomerJournalCap<Hooks> {
    type Tag = CustomerJournalTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for CustomerJournalCap<Hooks> {
    type Hooked = CustomerJournalCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the journal and sidebar registries (tail first = install order).
pub trait FoldCustomerJournalHooks {
    fn fold(
        self,
        registry: CustomerJournalRegistry,
        sidebar: CustomerSidebarRegistry,
    ) -> (CustomerJournalRegistry, CustomerSidebarRegistry);
}

impl FoldCustomerJournalHooks for HNil {
    fn fold(
        self,
        registry: CustomerJournalRegistry,
        sidebar: CustomerSidebarRegistry,
    ) -> (CustomerJournalRegistry, CustomerSidebarRegistry) {
        (registry, sidebar)
    }
}

impl<Plugin, H, Tail> FoldCustomerJournalHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldCustomerJournalHooks,
    H: CustomerJournalRegistrar + Copy,
{
    fn fold(
        self,
        registry: CustomerJournalRegistry,
        sidebar: CustomerSidebarRegistry,
    ) -> (CustomerJournalRegistry, CustomerSidebarRegistry) {
        let (registry, sidebar) = self.tail.fold(registry, sidebar);
        let hook = self.head.value;
        (
            hook.register_customer_journal(registry),
            hook.register_customer_sidebar(sidebar),
        )
    }
}

impl<Hooks> Capability for CustomerJournalCap<Hooks>
where
    Hooks: FoldCustomerJournalHooks,
{
    type Output = Tagged<CustomerJournalTag, CustomerJournalRegistry>;
    type Value = CustomerJournalRegistry;
    type Hooks = Hooks;
    type Items = CustomerJournalRegistry;

    fn mount(self) -> Self::Output {
        let (registry, sidebar) = self.hooks.fold(self.items, self.sidebar);
        store_customer_sidebar_links(&sidebar);
        Tagged::new(registry)
    }
}

/// Attach an empty customer journal capability.
pub fn with_customer_journal<L, Proof>(app: App<L>) -> App<HCons<CustomerJournalCap<HNil>, L>>
where
    L: HList + CapTagAbsent<CustomerJournalTag, Proof>,
{
    app.add_capability(CustomerJournalCap::<HNil>::new())
}
