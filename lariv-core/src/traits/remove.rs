//! Remove capabilities from the builder-phase HList by tag.

use frunk::{
    HCons,
    indices::{Here, There},
};

use crate::{app::App, capability::HasCapTag};

/// Pluck a builder capability by its [`HasCapTag::Tag`].
pub trait PluckByCapTag<Tag, Index> {
    type Value;
    type Remainder;

    fn pluck_by_cap_tag(self) -> (Self::Value, Self::Remainder);
}

impl<Tag, Cap, Tail> PluckByCapTag<Tag, Here> for HCons<Cap, Tail>
where
    Cap: HasCapTag<Tag = Tag>,
{
    type Value = Cap;
    type Remainder = Tail;

    fn pluck_by_cap_tag(self) -> (Self::Value, Self::Remainder) {
        (self.head, self.tail)
    }
}

impl<Head, Tail, Tag, TailIndex> PluckByCapTag<Tag, There<TailIndex>> for HCons<Head, Tail>
where
    Tail: PluckByCapTag<Tag, TailIndex>,
{
    type Value = <Tail as PluckByCapTag<Tag, TailIndex>>::Value;
    type Remainder = HCons<Head, <Tail as PluckByCapTag<Tag, TailIndex>>::Remainder>;

    fn pluck_by_cap_tag(self) -> (Self::Value, Self::Remainder) {
        let (target, tail_remainder) = self.tail.pluck_by_cap_tag();
        (
            target,
            HCons {
                head: self.head,
                tail: tail_remainder,
            },
        )
    }
}

/// Pluck a `Tagged<Tag, _>` from a mounted / item HList by tag.
pub trait PluckByTag<Tag, Index> {
    type Value;
    type Remainder;

    fn pluck_by_tag(self) -> (crate::tag::Tagged<Tag, Self::Value>, Self::Remainder);
}

impl<Tag, V, Tail> PluckByTag<Tag, Here> for HCons<crate::tag::Tagged<Tag, V>, Tail> {
    type Value = V;
    type Remainder = Tail;

    fn pluck_by_tag(self) -> (crate::tag::Tagged<Tag, Self::Value>, Self::Remainder) {
        (self.head, self.tail)
    }
}

impl<Head, Tail, Tag, TailIndex> PluckByTag<Tag, There<TailIndex>> for HCons<Head, Tail>
where
    Tail: PluckByTag<Tag, TailIndex>,
{
    type Value = <Tail as PluckByTag<Tag, TailIndex>>::Value;
    type Remainder = HCons<Head, <Tail as PluckByTag<Tag, TailIndex>>::Remainder>;

    fn pluck_by_tag(self) -> (crate::tag::Tagged<Tag, Self::Value>, Self::Remainder) {
        let (target, tail_remainder) = self.tail.pluck_by_tag();
        (
            target,
            HCons {
                head: self.head,
                tail: tail_remainder,
            },
        )
    }
}

pub trait RemoveCapability<Tag, Index> {
    type Output;
    fn remove_capability(self) -> Self::Output;
}

impl<L, Tag, Index> RemoveCapability<Tag, Index> for App<L>
where
    L: PluckByCapTag<Tag, Index>,
{
    type Output = App<<L as PluckByCapTag<Tag, Index>>::Remainder>;

    fn remove_capability(self) -> Self::Output {
        let (_, remainder) = self.capabilities.pluck_by_cap_tag();
        App {
            capabilities: remainder,
        }
    }
}

impl<L> App<L> {
    /// Remove the builder capability with tag `Tag`.
    pub fn remove_capability<Tag, Index>(self) -> App<<L as PluckByCapTag<Tag, Index>>::Remainder>
    where
        L: PluckByCapTag<Tag, Index>,
    {
        RemoveCapability::remove_capability(self)
    }
}

#[cfg(test)]
mod tests {
    use frunk::{HNil, hlist};

    use super::*;
    use crate::{
        capability::{CapStore, Capability},
        tag::Tagged,
        traits::add::AddCapability,
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
    fn removes_by_tag_only() {
        let app = App {
            capabilities: hlist![],
        }
        .add_capability(CapStore::<DbTag, HNil, _>::with_items("pg"))
        .add_capability(CapStore::<AuthTag, HNil, _>::with_items(true));

        let app = app.remove_capability::<AuthTag, _>();
        assert_eq!(app.capabilities.head.items, "pg");
    }
}
