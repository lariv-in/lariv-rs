use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::App,
    capability::{CapStore, Capability},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

macro_rules! zst_hook {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub struct $name<Plugin> {
            _plugin: ::std::marker::PhantomData<fn() -> Plugin>,
        }

        impl<Plugin> Clone for $name<Plugin> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<Plugin> Copy for $name<Plugin> {}

        impl<Plugin> Default for $name<Plugin> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<Plugin> $name<Plugin> {
            pub const fn new() -> Self {
                Self {
                    _plugin: ::std::marker::PhantomData,
                }
            }
        }
    };
}

pub(crate) use zst_hook;

zst_hook!(MountRoutesHook);
zst_hook!(WithStateHook);
zst_hook!(SeedHook);

/// Capability tag for deferred state-attachment hooks (applied during prep, before mount).
pub struct StateHooksTag;

/// Capability tag for deferred seed hooks (mounted as a runner; executed separately).
pub struct SeedsTag;

/// Builder-phase state hooks capability.
pub type StateHooksCap<Hooks> = CapStore<StateHooksTag, Hooks, HNil>;

impl<Hooks> StateHooksCap<Hooks> {
    pub fn add_with_state<Plugin>(self) -> StateHooksCap<HCons<Tagged<Plugin, WithStateHook<Plugin>>, Hooks>> {
        self.add_hook(WithStateHook::new())
    }
}

impl<Hooks> Capability for StateHooksCap<Hooks> {
    type Value = ();
    type Output = Tagged<StateHooksTag, ()>;
    type Hooks = Hooks;
    type Items = HNil;

    fn mount(self) -> Self::Output {
        // State hooks are applied during prep (`attach_states`); nothing left to mount.
        Tagged::new(())
    }
}

/// Seed runner holding queued [`SeedHook`]s (execution is fallible / async).
#[derive(Clone)]
pub struct SeedRunner<Seeds> {
    pub seeds: Seeds,
}

/// Builder-phase seeds capability.
pub type SeedsCap<Hooks> = CapStore<SeedsTag, Hooks, HNil>;

impl<Hooks> SeedsCap<Hooks> {
    pub fn add_seed<Plugin>(self) -> SeedsCap<HCons<Tagged<Plugin, SeedHook<Plugin>>, Hooks>> {
        self.add_hook(SeedHook::new())
    }
}

impl<Hooks> Capability for SeedsCap<Hooks> {
    type Value = SeedRunner<Hooks>;
    type Output = Tagged<SeedsTag, SeedRunner<Hooks>>;
    type Hooks = Hooks;
    type Items = HNil;

    fn mount(self) -> Self::Output {
        Tagged::new(SeedRunner { seeds: self.hooks })
    }
}

/// Plugin hook: attach runtime state after DB is connected.
pub trait AttachState<L, Proof = ()>: Sized {
    type Output;
    fn attach_state(app: App<L>) -> App<Self::Output>;
}

/// Fold [`WithStateHook`]s (tail first = install order).
pub trait FoldAttachState<L, Proof = ()>: Sized {
    type Output;
    fn fold_attach_state(self, app: App<L>) -> App<Self::Output>;
}

impl<L> FoldAttachState<L> for HNil {
    type Output = L;

    fn fold_attach_state(self, app: App<L>) -> App<Self::Output> {
        app
    }
}

impl<Plugin, Tail, L, TailProof, Proof> FoldAttachState<L, (TailProof, Proof)>
    for HCons<Tagged<Plugin, WithStateHook<Plugin>>, Tail>
where
    WithStateHook<Plugin>: AttachState<L, Proof>,
    Tail: FoldAttachState<<WithStateHook<Plugin> as AttachState<L, Proof>>::Output, TailProof>,
{
    type Output = <Tail as FoldAttachState<
        <WithStateHook<Plugin> as AttachState<L, Proof>>::Output,
        TailProof,
    >>::Output;

    fn fold_attach_state(self, app: App<L>) -> App<Self::Output> {
        let app = <WithStateHook<Plugin> as AttachState<L, Proof>>::attach_state(app);
        self.tail.fold_attach_state(app)
    }
}

/// Plugin hook: run startup seed against the fully built (mounted) app.
#[async_trait::async_trait]
pub trait RunSeed<M, Proof = ()>: Sized {
    async fn run_seed(app: &crate::app::MountedApp<M>) -> anyhow::Result<()>;
}

/// Fold [`SeedHook`]s (tail first = install order).
#[async_trait::async_trait]
pub trait FoldSeeds<M, Proof = ()>: Sized {
    async fn fold_seeds(self, app: &crate::app::MountedApp<M>) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<M> FoldSeeds<M> for HNil
where
    M: Sync,
{
    async fn fold_seeds(self, _app: &crate::app::MountedApp<M>) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl<Plugin, Tail, M, TailProof, Proof> FoldSeeds<M, (TailProof, Proof)>
    for HCons<Tagged<Plugin, SeedHook<Plugin>>, Tail>
where
    M: Sync,
    Plugin: Send + Sync + 'static,
    SeedHook<Plugin>: RunSeed<M, Proof> + Send,
    Tail: FoldSeeds<M, TailProof> + Send,
{
    async fn fold_seeds(self, app: &crate::app::MountedApp<M>) -> anyhow::Result<()> {
        self.tail.fold_seeds(app).await?;
        <SeedHook<Plugin> as RunSeed<M, Proof>>::run_seed(app).await
    }
}

pub fn with_state_hooks<L, Proof>(app: App<L>) -> App<HCons<StateHooksCap<HNil>, L>>
where
    L: HList + CapTagAbsent<StateHooksTag, Proof>,
{
    app.add_capability(CapStore::with_items(HNil))
}

pub fn with_seeds<L, Proof>(app: App<L>) -> App<HCons<SeedsCap<HNil>, L>>
where
    L: HList + CapTagAbsent<SeedsTag, Proof>,
{
    app.add_capability(CapStore::with_items(HNil))
}
