use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::App,
    capability::{CapStore, Capability},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for deferred state-attachment hooks (applied during prep, before mount).
pub struct StateHooksTag;

/// Capability tag for deferred seed hooks (mounted as a runner; executed separately).
pub struct SeedsTag;

/// Builder-phase state hooks capability.
pub type StateHooksCap<Hooks> = CapStore<StateHooksTag, Hooks, HNil>;

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

/// Seed runner holding queued seed hooks (execution is fallible / async).
#[derive(Clone)]
pub struct SeedRunner<Seeds> {
    pub seeds: Seeds,
}

/// Builder-phase seeds capability.
pub type SeedsCap<Hooks> = CapStore<SeedsTag, Hooks, HNil>;

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

/// Fold state hooks (tail first = install order).
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

impl<Plugin, Hook, Tail, L, TailProof, Proof> FoldAttachState<L, (TailProof, Proof)>
    for HCons<Tagged<Plugin, Hook>, Tail>
where
    Hook: AttachState<L, Proof>,
    Tail: FoldAttachState<<Hook as AttachState<L, Proof>>::Output, TailProof>,
{
    type Output = <Tail as FoldAttachState<
        <Hook as AttachState<L, Proof>>::Output,
        TailProof,
    >>::Output;

    fn fold_attach_state(self, app: App<L>) -> App<Self::Output> {
        let app = <Hook as AttachState<L, Proof>>::attach_state(app);
        self.tail.fold_attach_state(app)
    }
}

/// Plugin hook: run startup seed against the fully built (mounted) app.
#[async_trait::async_trait]
pub trait RunSeed<M, Proof = ()>: Sized {
    async fn run_seed(app: &crate::app::MountedApp<M>) -> anyhow::Result<()>;
}

/// Fold seed hooks (tail first = install order).
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
impl<Plugin, Hook, Tail, M, TailProof, Proof> FoldSeeds<M, (TailProof, Proof)>
    for HCons<Tagged<Plugin, Hook>, Tail>
where
    M: Sync,
    Plugin: Send + Sync + 'static,
    Hook: RunSeed<M, Proof> + Send,
    Tail: FoldSeeds<M, TailProof> + Send,
{
    async fn fold_seeds(self, app: &crate::app::MountedApp<M>) -> anyhow::Result<()> {
        self.tail.fold_seeds(app).await?;
        <Hook as RunSeed<M, Proof>>::run_seed(app).await
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
