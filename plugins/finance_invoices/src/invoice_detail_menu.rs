//! Extra entries on quotation, draft, posted, and cancelled invoice sidebars.
//!
//! The invoices plugin attaches this capability and a base hook that adds no links.
//! Addon plugins `cap_hook` it and push an [`InvoiceDetailLink`]. The folded list is
//! stored once at app mount and read by [`invoice_detail_links`].

use std::marker::PhantomData;
use std::sync::OnceLock;

use frunk::{HCons, HNil};
use lariv_core::app::App;
use lariv_core::capability::{CapHookExt, Capability, HasCapTag};
use lariv_core::tag::Tagged;
use lariv_core::traits::add::{AddCapability, CapTagAbsent};
use lariv_plugin_users::role_authorization::current_auth;
use lariv_plugin_users::roles::Superuser;

static LINKS: OnceLock<Vec<InvoiceDetailLink>> = OnceLock::new();

/// Which invoice detail sidebar a link is rendered on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvoiceDetailKind {
    Quotation,
    Draft,
    Posted,
    Cancelled,
}

impl InvoiceDetailKind {
    /// Postgres table whose header row this sidebar belongs to.
    pub fn table_name(self) -> &'static str {
        match self {
            Self::Quotation => "quotation_invoices",
            Self::Draft => "draft_invoices",
            Self::Posted => "posted_invoices",
            Self::Cancelled => "cancelled_invoices",
        }
    }

    /// Line-item table for this document.
    pub fn line_table_name(self) -> &'static str {
        match self {
            Self::Quotation => "quotation_invoice_lines",
            Self::Draft => "draft_invoice_lines",
            Self::Posted => "posted_invoice_lines",
            Self::Cancelled => "cancelled_invoice_lines",
        }
    }

    /// Column on [`Self::line_table_name`] that points at the header id.
    pub fn line_parent_column(self) -> &'static str {
        match self {
            Self::Quotation => "quotation_invoice_id",
            Self::Draft => "draft_invoice_id",
            Self::Posted => "posted_invoice_id",
            Self::Cancelled => "cancelled_invoice_id",
        }
    }
}

/// One extra entry on an invoice detail sidebar, after the built-in detail item.
#[derive(Clone, Debug)]
pub struct InvoiceDetailLink {
    /// Active-state key compared against the `active` argument of the detail menus.
    pub key: &'static str,
    pub title: &'static str,
    /// Builds the link target for one document of `kind`.
    pub url_fn: fn(InvoiceDetailKind, i64) -> String,
    pub order: u16,
    /// Hidden unless the effective user is a superuser.
    pub superuser_only: bool,
}

/// Invoice detail link registry folded from base and addon hooks.
#[derive(Clone, Debug, Default)]
pub struct InvoiceDetailMenuRegistry {
    links: Vec<InvoiceDetailLink>,
}

impl InvoiceDetailMenuRegistry {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// Add a link. A duplicate `key` is ignored.
    pub fn push(mut self, link: InvoiceDetailLink) -> Self {
        if !self.links.iter().any(|existing| existing.key == link.key) {
            self.links.push(link);
        }
        self
    }

    pub fn links(&self) -> &[InvoiceDetailLink] {
        &self.links
    }

    pub(crate) fn sorted_links(self) -> Vec<InvoiceDetailLink> {
        let mut links = self.links;
        links.sort_by_key(|link| (link.order, link.title));
        links
    }
}

/// Capability tag for the invoice detail sidebar registry.
pub struct InvoiceDetailMenuTag;

/// Builder-phase registry. Addon plugins prepend hooks with `cap_hook`.
#[derive(Clone, Default)]
pub struct InvoiceDetailMenuCap<Hooks> {
    pub hooks: Hooks,
    pub items: InvoiceDetailMenuRegistry,
    _tag: PhantomData<fn() -> InvoiceDetailMenuTag>,
}

impl<Hooks> InvoiceDetailMenuCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: InvoiceDetailMenuRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> InvoiceDetailMenuCap<HCons<Tagged<HTag, H>, Hooks>> {
        InvoiceDetailMenuCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for InvoiceDetailMenuCap<Hooks> {
    type Tag = InvoiceDetailMenuTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for InvoiceDetailMenuCap<Hooks> {
    type Hooked = InvoiceDetailMenuCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Plugin hook that contributes invoice-detail sidebar links.
pub trait InvoiceDetailMenuRegistrar: Sized {
    fn register_invoice_detail_links(
        self,
        cap: InvoiceDetailMenuRegistry,
    ) -> InvoiceDetailMenuRegistry;
}

/// Fold registrar hooks over the registry. Tail first, so install order wins.
pub trait FoldInvoiceDetailMenuHooks {
    fn fold(self, registry: InvoiceDetailMenuRegistry) -> InvoiceDetailMenuRegistry;
}

impl FoldInvoiceDetailMenuHooks for HNil {
    fn fold(self, registry: InvoiceDetailMenuRegistry) -> InvoiceDetailMenuRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldInvoiceDetailMenuHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldInvoiceDetailMenuHooks,
    H: InvoiceDetailMenuRegistrar + Copy,
{
    fn fold(self, registry: InvoiceDetailMenuRegistry) -> InvoiceDetailMenuRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_invoice_detail_links(registry)
    }
}

impl<Hooks> Capability for InvoiceDetailMenuCap<Hooks>
where
    Hooks: FoldInvoiceDetailMenuHooks,
{
    type Value = InvoiceDetailMenuRegistry;
    type Output = Tagged<InvoiceDetailMenuTag, InvoiceDetailMenuRegistry>;
    type Hooks = Hooks;
    type Items = InvoiceDetailMenuRegistry;

    fn mount(self) -> Self::Output {
        let registry = self.hooks.fold(self.items);
        store_invoice_detail_links(&registry);
        Tagged::new(registry)
    }
}

/// Attach an empty invoice-detail menu capability.
///
/// Prefer `cap_attach` in the finance invoices plugin install steps.
pub fn with_invoice_detail_menu<L, Proof>(app: App<L>) -> App<HCons<InvoiceDetailMenuCap<HNil>, L>>
where
    L: frunk::hlist::HList + CapTagAbsent<InvoiceDetailMenuTag, Proof>,
{
    app.add_capability(InvoiceDetailMenuCap::new())
}

fn store_invoice_detail_links(registry: &InvoiceDetailMenuRegistry) {
    if LINKS.set(registry.clone().sorted_links()).is_err() {
        tracing::error!("invoice detail menu LINKS already initialized");
    }
}

fn stored_links() -> Vec<InvoiceDetailLink> {
    match LINKS.get() {
        Some(links) => links.clone(),
        None => BaseHook
            .register_invoice_detail_links(InvoiceDetailMenuRegistry::new())
            .sorted_links(),
    }
}

/// Registered invoice-detail links visible to the effective user, sorted by order.
///
/// Links marked [`InvoiceDetailLink::superuser_only`] are omitted unless the
/// effective user is a superuser. Before app mount this falls back to the base
/// hook, which registers nothing.
pub fn invoice_detail_links() -> Vec<InvoiceDetailLink> {
    let links = stored_links();
    let superuser = current_auth().is_some_and(|auth| Superuser::matches(&auth.role));
    if superuser {
        links
    } else {
        links
            .into_iter()
            .filter(|link| !link.superuser_only)
            .collect()
    }
}

/// Base hook installed by the finance invoices plugin. Built-in items stay in the template.
#[derive(Clone, Copy, Default)]
pub struct BaseHook;

impl InvoiceDetailMenuRegistrar for BaseHook {
    fn register_invoice_detail_links(
        self,
        cap: InvoiceDetailMenuRegistry,
    ) -> InvoiceDetailMenuRegistry {
        cap
    }
}
