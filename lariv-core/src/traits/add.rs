//! Prepend capabilities and prove tag absence at compile time.

use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::App,
    capability::HasCapTag,
    tag::{CompileEq, CompileEqResult},
};

/// Witness that `Tag` does not appear as any capability's [`HasCapTag::Tag`].
pub trait CapTagAbsent<Tag, Proof> {}

impl<Tag> CapTagAbsent<Tag, ()> for HNil {}

impl<Tag, Head, Tail, HeadProof, TailProof> CapTagAbsent<Tag, (HeadProof, TailProof)>
    for HCons<Head, Tail>
where
    Head: HasCapTag,
    Tag: CompileEq<Head::Tag, HeadProof>,
    HeadProof: CompileEqResult,
    Tail: CapTagAbsent<Tag, TailProof>,
{
}

/// Prepend a capability, but only if its tag is not already present.
pub trait AddCapability<C, Proof>
where
    C: HasCapTag,
{
    type Output;
    fn add_capability(self, cap: C) -> Self::Output;
}

impl<C, L, Proof> AddCapability<C, Proof> for App<L>
where
    C: HasCapTag,
    L: HList + CapTagAbsent<C::Tag, Proof>,
{
    type Output = App<HCons<C, L>>;

    fn add_capability(self, cap: C) -> Self::Output {
        App {
            capabilities: HCons {
                head: cap,
                tail: self.capabilities,
            },
        }
    }
}

/// Legacy alias used by older call sites — prefer [`CapTagAbsent`].
pub trait TagAbsent<Tag, Proof>: CapTagAbsent<Tag, Proof> {}
impl<T, Tag, Proof> TagAbsent<Tag, Proof> for T where T: CapTagAbsent<Tag, Proof> {}

#[cfg(test)]
mod tests {
    use frunk::{HNil, hlist};

    use super::*;
    use crate::{
        capability::{CapStore, Capability},
        tag::Tagged,
    };

    struct AuthTag;
    struct DbTag;

    impl Capability for CapStore<AuthTag, HNil, bool> {
        type Value = bool;
        type Output = Tagged<AuthTag, bool>;
        type Hooks = HNil;
        type Items = bool;

        fn mount(self) -> Self::Output {
            Tagged::new(self.items)
        }
    }

    impl Capability for CapStore<DbTag, HNil, &'static str> {
        type Value = &'static str;
        type Output = Tagged<DbTag, &'static str>;
        type Hooks = HNil;
        type Items = &'static str;

        fn mount(self) -> Self::Output {
            Tagged::new(self.items)
        }
    }

    #[test]
    fn adds_unique_tags() {
        let app = App {
            capabilities: hlist![],
        }
        .add_capability(CapStore::<DbTag, HNil, _>::with_items("pg"))
        .add_capability(CapStore::<AuthTag, HNil, _>::with_items(true));

        assert!(app.capabilities.head.items);
        assert_eq!(app.capabilities.tail.head.items, "pg");
    }
}
