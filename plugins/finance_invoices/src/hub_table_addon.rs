//! Extra columns on the invoice hub list table, registered by other plugins.
//!
//! Invoices attaches [`InvoiceHubTableCap`]; deployments and sibling plugins
//! register columns with `cap_hook(InvoiceHubTableTag, InvoiceHubTableCap, Hook)`.
//! Handlers extract [`Cap`](lariv_core::http::Cap)`<`[`InvoiceHubTableRegistry`]`>`.

use std::collections::HashMap;
use std::marker::PhantomData;

use async_trait::async_trait;
use frunk::{HCons, HNil, hlist::HList};
use lariv_core::app::App;
use lariv_core::capability::{CapHookExt, Capability, HasCapTag};
use lariv_core::tag::Tagged;
use lariv_core::traits::add::{AddCapability, CapTagAbsent};
use sea_orm::DatabaseConnection;

use crate::invoice_addon_key::InvoiceAddonKey;

/// Capability tag for the invoice hub table column registry.
pub struct InvoiceHubTableTag;

/// One plugin's extra column on the invoice hub table.
#[async_trait]
pub trait InvoiceHubTableAddon: Send + Sync {
    fn id(&self) -> &'static str;

    /// Stable column key for client-side column visibility.
    fn column_key(&self) -> &'static str;

    /// Header label shown in the table.
    fn column_label(&self) -> &'static str;

    /// Cell text keyed by [`InvoiceAddonKey`]. Missing keys render empty.
    async fn cell_values(
        &self,
        db: &DatabaseConnection,
        keys: &[InvoiceAddonKey],
    ) -> HashMap<InvoiceAddonKey, String>;
}

/// Folded list of hub-table column addons (mounted capability value).
#[derive(Clone, Default)]
pub struct InvoiceHubTableRegistry {
    addons: Vec<&'static dyn InvoiceHubTableAddon>,
}

impl InvoiceHubTableRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a column addon; duplicate ids are ignored (install-order first wins).
    pub fn register_addon(mut self, addon: &'static dyn InvoiceHubTableAddon) -> Self {
        if !self
            .addons
            .iter()
            .any(|existing| existing.id() == addon.id())
        {
            self.addons.push(addon);
        }
        self
    }

    pub fn addons(&self) -> &[&'static dyn InvoiceHubTableAddon] {
        &self.addons
    }
}

/// Plugin hook for registering hub-table columns at install time.
pub trait InvoiceHubTableRegistrar: Sized {
    fn register_invoice_hub_table(
        self,
        registry: InvoiceHubTableRegistry,
    ) -> InvoiceHubTableRegistry;
}

/// Builder-phase invoice hub table capability.
#[derive(Clone, Default)]
pub struct InvoiceHubTableCap<Hooks> {
    pub hooks: Hooks,
    pub items: InvoiceHubTableRegistry,
    _tag: PhantomData<fn() -> InvoiceHubTableTag>,
}

impl<Hooks> InvoiceHubTableCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: InvoiceHubTableRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> InvoiceHubTableCap<HCons<Tagged<HTag, H>, Hooks>> {
        InvoiceHubTableCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for InvoiceHubTableCap<Hooks> {
    type Tag = InvoiceHubTableTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for InvoiceHubTableCap<Hooks> {
    type Hooked = InvoiceHubTableCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the registry (tail first = install order).
pub trait FoldInvoiceHubTableRegistrarHooks {
    fn fold(self, registry: InvoiceHubTableRegistry) -> InvoiceHubTableRegistry;
}

impl FoldInvoiceHubTableRegistrarHooks for HNil {
    fn fold(self, registry: InvoiceHubTableRegistry) -> InvoiceHubTableRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldInvoiceHubTableRegistrarHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldInvoiceHubTableRegistrarHooks,
    H: InvoiceHubTableRegistrar + Copy,
{
    fn fold(self, registry: InvoiceHubTableRegistry) -> InvoiceHubTableRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_invoice_hub_table(registry)
    }
}

impl<Hooks> Capability for InvoiceHubTableCap<Hooks>
where
    Hooks: FoldInvoiceHubTableRegistrarHooks,
{
    type Value = InvoiceHubTableRegistry;
    type Output = Tagged<InvoiceHubTableTag, InvoiceHubTableRegistry>;
    type Hooks = Hooks;
    type Items = InvoiceHubTableRegistry;

    fn mount(self) -> Self::Output {
        let registry = self.hooks.fold(self.items);
        Tagged::new(registry)
    }
}

/// Attach an empty hub-table capability (prefer `cap_attach` in install steps).
pub fn with_invoice_hub_table<L, Proof>(app: App<L>) -> App<HCons<InvoiceHubTableCap<HNil>, L>>
where
    L: HList + CapTagAbsent<InvoiceHubTableTag, Proof>,
{
    app.add_capability(InvoiceHubTableCap::<HNil>::new())
}

/// Header metadata for one registered extra column.
#[derive(Clone, Debug)]
pub struct InvoiceHubExtraColumn {
    pub key: &'static str,
    pub label: &'static str,
}

/// Fill [`super::templates::InvoiceRow::extra_cells`] for all registered addons.
///
/// Returns column headers in registration order (same order as each row's
/// `extra_cells`).
pub async fn enrich_hub_rows(
    db: &DatabaseConnection,
    registry: &InvoiceHubTableRegistry,
    rows: &mut [super::templates::InvoiceRow],
) -> Vec<InvoiceHubExtraColumn> {
    let registered = registry.addons();
    let columns: Vec<InvoiceHubExtraColumn> = registered
        .iter()
        .map(|addon| InvoiceHubExtraColumn {
            key: addon.column_key(),
            label: addon.column_label(),
        })
        .collect();

    if columns.is_empty() {
        for row in rows.iter_mut() {
            row.extra_cells.clear();
        }
        return columns;
    }

    let mut keys: Vec<InvoiceAddonKey> = rows.iter().filter_map(|r| r.addon_key).collect();
    keys.sort_by_key(|k| (k.is_posted(), k.id()));
    keys.dedup();

    let mut maps = Vec::with_capacity(registered.len());
    for addon in registered {
        maps.push(addon.cell_values(db, &keys).await);
    }

    for row in rows.iter_mut() {
        row.extra_cells = maps
            .iter()
            .map(|m| {
                row.addon_key
                    .and_then(|key| m.get(&key).cloned())
                    .unwrap_or_default()
            })
            .collect();
    }

    columns
}
