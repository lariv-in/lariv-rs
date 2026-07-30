use frunk::{
    HCons,
    indices::{Here, There},
};

use crate::{app::App, capability::HasCapTag};

/// Map a builder capability in place by its [`HasCapTag::Tag`].
pub trait MapByCapTag<Tag, NewCap, Index> {
    type OldValue;
    type Output;

    fn map_by_cap_tag<F>(self, f: F) -> Self::Output
    where
        F: FnOnce(Self::OldValue) -> NewCap;
}

impl<Tag, Cap, NewCap, Tail> MapByCapTag<Tag, NewCap, Here> for HCons<Cap, Tail>
where
    Cap: HasCapTag<Tag = Tag>,
{
    type OldValue = Cap;
    type Output = HCons<NewCap, Tail>;

    fn map_by_cap_tag<F>(self, f: F) -> Self::Output
    where
        F: FnOnce(Self::OldValue) -> NewCap,
    {
        HCons {
            head: f(self.head),
            tail: self.tail,
        }
    }
}

impl<Head, Tail, Tag, NewCap, TailIndex> MapByCapTag<Tag, NewCap, There<TailIndex>>
    for HCons<Head, Tail>
where
    Tail: MapByCapTag<Tag, NewCap, TailIndex>,
{
    type OldValue = <Tail as MapByCapTag<Tag, NewCap, TailIndex>>::OldValue;
    type Output = HCons<Head, <Tail as MapByCapTag<Tag, NewCap, TailIndex>>::Output>;

    fn map_by_cap_tag<F>(self, f: F) -> Self::Output
    where
        F: FnOnce(Self::OldValue) -> NewCap,
    {
        HCons {
            head: self.head,
            tail: self.tail.map_by_cap_tag(f),
        }
    }
}

/// Map a `Tagged<Tag, _>` value in a mounted HList (kept for nested item HLists).
pub trait MapByTag<Tag, NewValue, Index> {
    type OldValue;
    type Output;

    fn map_by_tag<F>(self, f: F) -> Self::Output
    where
        F: FnOnce(Self::OldValue) -> NewValue;
}

impl<Tag, OldValue, NewValue, Tail> MapByTag<Tag, NewValue, Here>
    for HCons<crate::tag::Tagged<Tag, OldValue>, Tail>
{
    type OldValue = OldValue;
    type Output = HCons<crate::tag::Tagged<Tag, NewValue>, Tail>;

    fn map_by_tag<F>(self, f: F) -> Self::Output
    where
        F: FnOnce(Self::OldValue) -> NewValue,
    {
        HCons {
            head: crate::tag::Tagged::new(f(self.head.value)),
            tail: self.tail,
        }
    }
}

impl<Head, Tail, Tag, NewValue, TailIndex> MapByTag<Tag, NewValue, There<TailIndex>>
    for HCons<Head, Tail>
where
    Tail: MapByTag<Tag, NewValue, TailIndex>,
{
    type OldValue = <Tail as MapByTag<Tag, NewValue, TailIndex>>::OldValue;
    type Output = HCons<Head, <Tail as MapByTag<Tag, NewValue, TailIndex>>::Output>;

    fn map_by_tag<F>(self, f: F) -> Self::Output
    where
        F: FnOnce(Self::OldValue) -> NewValue,
    {
        HCons {
            head: self.head,
            tail: self.tail.map_by_tag(f),
        }
    }
}

pub trait ReplaceCapability<Tag, NewCap, Index> {
    type OldValue;
    type Output;

    fn replace_capability<F>(self, f: F) -> Self::Output
    where
        F: FnOnce(Self::OldValue) -> NewCap;
}

impl<L, Tag, NewCap, Index> ReplaceCapability<Tag, NewCap, Index> for App<L>
where
    L: MapByCapTag<Tag, NewCap, Index>,
{
    type OldValue = <L as MapByCapTag<Tag, NewCap, Index>>::OldValue;
    type Output = App<<L as MapByCapTag<Tag, NewCap, Index>>::Output>;

    fn replace_capability<F>(self, f: F) -> Self::Output
    where
        F: FnOnce(Self::OldValue) -> NewCap,
    {
        App {
            capabilities: self.capabilities.map_by_cap_tag(f),
        }
    }
}

impl<L> App<L> {
    /// Replace the builder capability with tag `Tag`.
    pub fn replace_capability<Tag, Index, NewCap>(
        self,
        f: impl FnOnce(<L as MapByCapTag<Tag, NewCap, Index>>::OldValue) -> NewCap,
    ) -> App<<L as MapByCapTag<Tag, NewCap, Index>>::Output>
    where
        L: MapByCapTag<Tag, NewCap, Index>,
    {
        ReplaceCapability::replace_capability(self, f)
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

    impl Capability for CapStore<AuthTag, HNil, u32> {
        type Value = u32;
        type Output = Tagged<AuthTag, u32>;
        type Hooks = HNil;
        type Items = u32;

        fn mount(self) -> Self::Output {
            Tagged::new(self.items)
        }
    }

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
    fn replaces_value_by_tag() {
        let app = App {
            capabilities: hlist![],
        }
        .add_capability(CapStore::<DbTag, HNil, _>::with_items("pg"))
        .add_capability(CapStore::<AuthTag, HNil, _>::with_items(1u32));

        let app = app.replace_capability::<AuthTag, _, _>(|cap| {
            CapStore::<AuthTag, HNil, _>::with_items(cap.items + 1)
        });
        assert_eq!(app.capabilities.head.items, 2);
        assert_eq!(app.capabilities.tail.head.items, "pg");
    }

    #[test]
    fn can_change_value_type() {
        let app = App {
            capabilities: hlist![],
        }
        .add_capability(CapStore::<AuthTag, HNil, _>::with_items(1u32));

        let app = app.replace_capability::<AuthTag, _, _>(|cap| {
            CapStore::<AuthTag, HNil, _>::with_items(cap.items > 0)
        });
        assert!(app.capabilities.head.items);
    }
}
