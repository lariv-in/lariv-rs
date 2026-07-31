use frunk::{
    HCons,
    indices::{Here, There},
};

use crate::{
    app::{App, MountedApp},
    capability::HasCapTag,
    tag::Tagged,
    traits::replace::MapByTag,
};

/// Borrow a builder capability by its [`HasCapTag::Tag`].
pub trait GetByCapTag<Tag, Index> {
    type Value;

    fn get_by_cap_tag(&self) -> &Self::Value;
}

impl<Tag, Cap, Tail> GetByCapTag<Tag, Here> for HCons<Cap, Tail>
where
    Cap: HasCapTag<Tag = Tag>,
{
    type Value = Cap;

    fn get_by_cap_tag(&self) -> &Self::Value {
        &self.head
    }
}

impl<Head, Tail, Tag, TailIndex> GetByCapTag<Tag, There<TailIndex>> for HCons<Head, Tail>
where
    Tail: GetByCapTag<Tag, TailIndex>,
{
    type Value = <Tail as GetByCapTag<Tag, TailIndex>>::Value;

    fn get_by_cap_tag(&self) -> &Self::Value {
        self.tail.get_by_cap_tag()
    }
}

/// Marks that `Tag` appears at `Index` in an template/item HList (search head-to-tail).
pub trait IndexOfTemplateTag<Tag, Index> {}

impl<Tag, V, Tail> IndexOfTemplateTag<Tag, Here> for HCons<Tagged<Tag, V>, Tail> {}

impl<Head, Tail, Tag, TailIndex> IndexOfTemplateTag<Tag, There<TailIndex>> for HCons<Head, Tail>
where
    Tail: IndexOfTemplateTag<Tag, TailIndex>,
{
}

/// Template HList with `Tag` mapped to `NewValue` at `Index`.
pub type ReplaceTemplateAtTag<Templates, Tag, NewValue, Index> = <Templates as MapByTag<
    Tag,
    NewValue,
    Index,
>>::Output;

/// Borrow a `Tagged<Tag, _>` value from a mounted HList by tag type alone.
pub trait GetByTag<Tag, Index> {
    type Value;

    fn get_by_tag(&self) -> &Self::Value;
}

impl<Tag, V, Tail> GetByTag<Tag, Here> for HCons<Tagged<Tag, V>, Tail> {
    type Value = V;

    fn get_by_tag(&self) -> &Self::Value {
        &self.head.value
    }
}

impl<Head, Tail, Tag, TailIndex> GetByTag<Tag, There<TailIndex>> for HCons<Head, Tail>
where
    Tail: GetByTag<Tag, TailIndex>,
{
    type Value = <Tail as GetByTag<Tag, TailIndex>>::Value;

    fn get_by_tag(&self) -> &Self::Value {
        self.tail.get_by_tag()
    }
}

impl<L> App<L> {
    /// Borrow the builder capability with tag `Tag`.
    pub fn get_capability<Tag, Index>(&self) -> &<L as GetByCapTag<Tag, Index>>::Value
    where
        L: GetByCapTag<Tag, Index>,
    {
        self.capabilities.get_by_cap_tag()
    }
}

impl<M> MountedApp<M> {
    /// Borrow the mounted capability output with tag `Tag`.
    pub fn get_capability_output<Tag, Index>(&self) -> &<M as GetByTag<Tag, Index>>::Value
    where
        M: GetByTag<Tag, Index>,
    {
        self.capabilities.get_by_tag()
    }
}

#[cfg(test)]
mod tests {
    use frunk::{HNil, hlist};

    use super::*;
    use crate::{
        capability::{CapStore, Capability},
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
    fn gets_by_cap_tag() {
        let app = App {
            capabilities: hlist![],
        }
        .add_capability(CapStore::<DbTag, HNil, _>::with_items("pg"))
        .add_capability(CapStore::<AuthTag, HNil, _>::with_items(true));

        assert!(app.get_capability::<AuthTag, _>().items);
        assert_eq!(app.get_capability::<DbTag, _>().items, "pg");
    }

    #[test]
    fn gets_mounted_by_tag() {
        let mounted = MountedApp {
            capabilities: hlist![Tagged::<AuthTag, _>::new(true), Tagged::<DbTag, _>::new("pg")],
        };
        assert!(*mounted.get_capability_output::<AuthTag, _>());
        assert_eq!(*mounted.get_capability_output::<DbTag, _>(), "pg");
    }

    #[test]
    fn index_of_template_tag_finds_head_and_tail() {
        struct LoginTag;
        struct HomeTag;

        type Pages = HCons<
            Tagged<HomeTag, u8>,
            HCons<Tagged<LoginTag, u16>, HNil>,
        >;

        fn assert_indices<T>()
        where
            T: IndexOfTemplateTag<HomeTag, Here>,
            T: IndexOfTemplateTag<LoginTag, There<Here>>,
        {
        }

        assert_indices::<Pages>();
    }
}
