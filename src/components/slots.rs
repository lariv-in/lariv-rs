//! Compile-time chrome slots (head / topbar / right-sidebar), parallel to templates.
//!
//! Plugins register contributors with [`SlotCapability::add`]. At serve time those
//! contributors are folded into a concrete [`ShellChrome`] (via
//! [`Cap`](crate::http::Cap)`<SlotCapability<_>>` in request extensions) and passed
//! into [`crate::template::RenderTemplate::render`].

use std::marker::PhantomData;
use std::sync::RwLock;

use frunk::{HCons, HNil, hlist::HList};
use maud::{Markup, html};

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, FoldRegistrarHooks, apply_registrar_hook, mount_with_hooks},
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByTag,
        replace::MapByTag,
    },
};

/// Capability tag for the slot registry.
pub struct SlotTag;

/// Kind tags for chrome slot groups (mirrors Go Catalog chrome).
pub struct HeadSlotTag;
pub struct TopbarItemsSlotTag;
pub struct RightSidebarSlotTag;

/// Optional context available when folding slots for a request.
#[derive(Clone, Debug, Default)]
pub struct SlotCtx {
    pub name: Option<String>,
    pub role: Option<String>,
    pub is_superuser: bool,
    pub is_staff: bool,
}

/// Folded chrome markup injected into shells by page renders.
#[derive(Clone)]
pub struct ShellChrome {
    pub head: Markup,
    pub topbar_items: Markup,
    pub right_sidebar: Markup,
}

impl Default for ShellChrome {
    fn default() -> Self {
        // Mirror `CoreTitle` so unit tests that skip slot folding still get a `<title>`.
        Self {
            head: html! {
                title { (document_title()) }
            },
            topbar_items: Markup::default(),
            right_sidebar: Markup::default(),
        }
    }
}

/// A contributor that produces chrome markup for a slot.
pub trait RenderSlot {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup;
}

/// Type marker registering a slot contributor `T` under identity `Tag` and bucket `Kind`.
pub struct SlotOf<Kind, T> {
    _kind: PhantomData<Kind>,
    _slot: PhantomData<fn() -> T>,
}

impl<Kind, T> Clone for SlotOf<Kind, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Kind, T> Copy for SlotOf<Kind, T> {}

impl<Kind, T> Default for SlotOf<Kind, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Kind, T> SlotOf<Kind, T> {
    pub const fn new() -> Self {
        Self {
            _kind: PhantomData,
            _slot: PhantomData,
        }
    }
}

/// Which [`ShellChrome`] field a slot kind appends into.
pub trait SlotBucket {
    fn push(chrome: &mut ShellChrome, markup: Markup);
}

impl SlotBucket for HeadSlotTag {
    fn push(chrome: &mut ShellChrome, markup: Markup) {
        chrome.head = html! {
            (chrome.head.clone())
            (markup)
        };
    }
}

impl SlotBucket for TopbarItemsSlotTag {
    fn push(chrome: &mut ShellChrome, markup: Markup) {
        chrome.topbar_items = html! {
            (chrome.topbar_items.clone())
            (markup)
        };
    }
}

impl SlotBucket for RightSidebarSlotTag {
    fn push(chrome: &mut ShellChrome, markup: Markup) {
        chrome.right_sidebar = html! {
            (chrome.right_sidebar.clone())
            (markup)
        };
    }
}

/// Fold an HList of [`SlotOf`] markers into [`ShellChrome`].
pub trait FoldSlots {
    fn fold_chrome(&self, ctx: &SlotCtx) -> ShellChrome;
}

impl FoldSlots for HNil {
    fn fold_chrome(&self, _: &SlotCtx) -> ShellChrome {
        ShellChrome::default()
    }
}

impl<Tag, Kind, T, Tail> FoldSlots for HCons<Tagged<Tag, SlotOf<Kind, T>>, Tail>
where
    Kind: SlotBucket,
    T: RenderSlot + Default,
    Tail: FoldSlots,
{
    fn fold_chrome(&self, ctx: &SlotCtx) -> ShellChrome {
        // Tail first so earlier registrations (deeper in the list) render first.
        let mut chrome = self.tail.fold_chrome(ctx);
        Kind::push(&mut chrome, T::default().render_slot(ctx));
        chrome
    }
}

/// Slot capability: a compile-time HList of tagged [`SlotOf`] markers.
///
/// Handlers extract [`crate::http::Cap`]`<SlotCapability<Slots>>`, fold with a
/// per-request [`SlotCtx`], and pass the resulting [`ShellChrome`] into page renders.
#[derive(Clone)]
pub struct SlotCapability<Slots> {
    pub slots: Slots,
}

type SlotCapabilityAfterAdd<Tag, Kind, T, Slots> =
    SlotCapability<HCons<Tagged<Tag, SlotOf<Kind, T>>, Slots>>;

impl SlotCapability<HNil> {
    pub fn new() -> Self {
        Self { slots: HNil }
    }
}

impl Default for SlotCapability<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Slots> SlotCapability<Slots> {
    /// Prepend a tagged slot marker (`Tag` = identity for replace; `Kind` = chrome bucket).
    pub fn add<Tag, Kind, T>(self) -> SlotCapabilityAfterAdd<Tag, Kind, T, Slots>
    where
        Slots: HList,
        Kind: SlotBucket,
        T: RenderSlot + Default + 'static,
    {
        SlotCapability {
            slots: HCons {
                head: Tagged::new(SlotOf::new()),
                tail: self.slots,
            },
        }
    }

    /// Borrow the slot marker stored under `Tag`.
    pub fn get_slot<Tag, Index>(&self) -> &<Slots as GetByTag<Tag, Index>>::Value
    where
        Slots: GetByTag<Tag, Index>,
    {
        self.slots.get_by_tag()
    }

    /// Replace the marker under `Tag`.
    pub fn replace_slot<Tag, Index, NewValue>(
        self,
        f: impl FnOnce(<Slots as MapByTag<Tag, NewValue, Index>>::OldValue) -> NewValue,
    ) -> SlotCapability<<Slots as MapByTag<Tag, NewValue, Index>>::Output>
    where
        Slots: MapByTag<Tag, NewValue, Index>,
    {
        SlotCapability {
            slots: self.slots.map_by_tag(f),
        }
    }

    /// Fold all registered contributors into chrome for `ctx`.
    pub fn fold_chrome(&self, ctx: &SlotCtx) -> ShellChrome
    where
        Slots: FoldSlots,
    {
        self.slots.fold_chrome(ctx)
    }

    /// Alias for [`fold_chrome`](Self::fold_chrome) used by page helpers.
    pub fn fold(&self, ctx: &SlotCtx) -> ShellChrome
    where
        Slots: FoldSlots,
    {
        self.fold_chrome(ctx)
    }
}

/// Plugin hook for appending slot markers onto a [`SlotCapability`].
pub trait SlotRegistrar<T>: Sized {
    type Output;
    fn register_slots(self, cap: SlotCapability<T>) -> SlotCapability<Self::Output>;
}

/// Builder-phase slot capability.
pub type SlotCap<Hooks, Items> = CapStore<SlotTag, Hooks, Items>;

impl<Hooks, Items> SlotCap<Hooks, Items> {
    pub fn resolve_hooks(self) -> SlotCap<HNil, <Hooks as FoldRegistrarHooks<SlotTag, Items>>::Output>
    where
        Hooks: FoldRegistrarHooks<SlotTag, Items>,
    {
        CapStore::with_items(self.hooks.fold_registrar_hooks(self.items))
    }
}

apply_registrar_hook! {
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    field: slots;
    proof: crate::capability::SlotHookProof;
    tag: SlotTag;
}

impl<Hooks, Items> Capability for SlotCap<Hooks, Items>
where
    Hooks: ApplyHooks<Items>,
{
    type Value = SlotCapability<Hooks::Output>;
    type Output = Tagged<SlotTag, SlotCapability<Hooks::Output>>;
    type Hooks = Hooks;
    type Items = Items;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, |items| SlotCapability { slots: items })
    }
}

// --- core.Title (Go `shellHeadTitle`) -------------------------------------------------

static DOCUMENT_TITLE: RwLock<String> = RwLock::new(String::new());

/// Current document title (`PWA_APP_NAME` when set, otherwise `"Lariv"`).
pub fn document_title() -> String {
    DOCUMENT_TITLE
        .read()
        .ok()
        .map(|g| g.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Lariv".into())
}

/// Set the document title (empty string restores the `"Lariv"` default).
pub fn set_document_title(title: impl Into<String>) {
    if let Ok(mut guard) = DOCUMENT_TITLE.write() {
        *guard = title.into();
    }
}

/// Slot identity for the core `<title>` head node (Go `"core.Title"`).
pub struct CoreTitleTag;

/// Default `<title>` head contributor.
#[derive(Default)]
pub struct CoreTitle;

impl RenderSlot for CoreTitle {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        let title = document_title();
        html! {
            title { (title) }
        }
    }
}

type DefaultSlots = HCons<Tagged<CoreTitleTag, SlotOf<HeadSlotTag, CoreTitle>>, HNil>;

pub fn with_slots<L, Proof>(app: App<L>) -> App<HCons<SlotCap<HNil, DefaultSlots>, L>>
where
    L: HList + CapTagAbsent<SlotTag, Proof>,
{
    app.add_capability(CapStore::with_items(
        SlotCapability::new()
            .add::<CoreTitleTag, HeadSlotTag, CoreTitle>()
            .slots,
    ))
}
