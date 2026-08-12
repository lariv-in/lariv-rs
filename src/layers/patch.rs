//! Query patcher HLists — composable pre-hooks without `dyn`.
//!
//! Patchers fold over HLists at compile time. Query patchers modify SeaORM-style builders
//! before load.
//!
//! # Use cases
//!
//! - Preload associations or apply tenant scopes on detail/list queries.
//!
//! # Examples
//!
//! ```rust ignore
//! DetailLayer::<UserLoader, UserTag>::new() // loader applies FoldQueryPatchers internally
//! ```

use frunk::HNil;

/// Modify a SeaORM-style query builder before execution.
pub trait QueryPatcher<Q> {
    fn patch_query(&self, query: Q) -> Q;
}

/// Fold an HList of query patchers.
pub trait FoldQueryPatchers<Q> {
    fn apply_query(&self, query: Q) -> Q;
}

impl<Q> FoldQueryPatchers<Q> for HNil {
    fn apply_query(&self, query: Q) -> Q {
        query
    }
}

impl<Head, Tail, Q> FoldQueryPatchers<Q> for frunk::HCons<Head, Tail>
where
    Head: QueryPatcher<Q>,
    Tail: FoldQueryPatchers<Q>,
{
    fn apply_query(&self, query: Q) -> Q {
        let query = self.tail.apply_query(query);
        self.head.patch_query(query)
    }
}
