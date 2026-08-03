//! Query and form patcher HLists — composable pre-hooks without `dyn`.
//!
//! Patchers fold over HLists at compile time. Query patchers modify SeaORM-style builders
//! before load; form patchers validate or normalize parsed form fields before create/update.
//!
//! # Use cases
//!
//! - Preload associations or apply tenant scopes on detail/list queries.
//! - Required-field validation and cross-field checks on create/update forms.
//!
//! # Examples
//!
//! ```rust ignore
//! DetailLayer::<UserLoader, UserTag>::new() // loader applies FoldQueryPatchers internally
//!
//! CreateLayer::<UserCreator, _>::new("/users/")
//!     .with_patchers(hlist![RequiredField("email"), UniqueEmail])
//! ```

use frunk::HNil;

/// Modify a SeaORM-style query builder before execution.
pub trait QueryPatcher<Q> {
    fn patch_query(&self, query: Q) -> Q;
}

/// Modify parsed form values / errors before create/update.
pub trait FormPatcher {
    fn patch_form(
        &self,
        values: &mut std::collections::HashMap<String, String>,
        errors: &mut std::collections::HashMap<String, String>,
    );
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

/// Fold an HList of form patchers.
pub trait FoldFormPatchers {
    fn apply_form(
        &self,
        values: &mut std::collections::HashMap<String, String>,
        errors: &mut std::collections::HashMap<String, String>,
    );
}

impl FoldFormPatchers for HNil {
    fn apply_form(
        &self,
        _values: &mut std::collections::HashMap<String, String>,
        _errors: &mut std::collections::HashMap<String, String>,
    ) {
    }
}

impl<Head, Tail> FoldFormPatchers for frunk::HCons<Head, Tail>
where
    Head: FormPatcher,
    Tail: FoldFormPatchers,
{
    fn apply_form(
        &self,
        values: &mut std::collections::HashMap<String, String>,
        errors: &mut std::collections::HashMap<String, String>,
    ) {
        self.tail.apply_form(values, errors);
        self.head.patch_form(values, errors);
    }
}
