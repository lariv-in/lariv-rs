//! Type-level tagging for HList items and compile-time type equality.
//!
//! [`Tagged`] wraps a value with a phantom tag type, enabling lookup by tag rather than
//! position in an HList. This is the core mechanism for capability registries, template
//! pages, route tags, and plugin state.
//!
//! # Examples
//!
//! ```rust
//! use lariv_rs::tag::Tagged;
//!
//! struct UsersTag;
//! let state = Tagged::<UsersTag, _>::new(42);
//! assert_eq!(state.value, 42);
//! ```

use std::marker::PhantomData;

/// Phantom-tag wrapper associating a value with a compile-time tag type.
///
/// Used throughout lariv-rs for capability outputs, template markers, route tags,
/// and plugin state. The tag type carries no runtime data.
///
/// # Use cases
///
/// - Mounted capability values keyed by plugin tag (e.g. `Tagged<UsersTag, UsersState>`)
/// - Template page registration in an HList
/// - Route URL builders keyed by route tag types
///
/// # Examples
///
/// ```rust
/// use lariv_rs::tag::Tagged;
///
/// struct MyTag;
/// let tagged = Tagged::<MyTag, &str>::new("hello");
/// assert_eq!(tagged.value, "hello");
/// ```
pub struct Tagged<T, V> {
    /// The wrapped value.
    pub value: V,
    tag: PhantomData<T>,
}

impl<T, V> Tagged<T, V> {
    /// Wrap `value` with phantom tag `T`.
    pub fn new(value: V) -> Self {
        Self {
            value,
            tag: PhantomData,
        }
    }
}

impl<T, V: Clone> Clone for Tagged<T, V> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            tag: PhantomData,
        }
    }
}

/// Witness that two types are the same (see [`CompileEq`]).
pub struct TypesEq;
/// Witness that two types differ (see [`CompileEq`]).
pub struct TypesNotEq;

mod sealed {
    pub trait CompileEqResult {}
    impl CompileEqResult for super::TypesEq {}
    impl CompileEqResult for super::TypesNotEq {}
}

pub use sealed::CompileEqResult;

/// Compile-time type equality probe.
///
/// - `T: CompileEq<T, TypesEq>` always holds.
/// - `T: CompileEq<U, TypesNotEq>` holds for any `T`, `U` (including `T == U`).
///
/// Leaving the result parameter inferred is unambiguous only when the types
/// differ; when they are equal both results apply and inference fails. That is
/// the usual way to require inequality in where-clauses.
///
/// # Use cases
///
/// - [`crate::traits::add::CapTagAbsent`] — prove a capability tag is not already present
/// - Preventing duplicate plugin tags at compile time
pub trait CompileEq<U, Result: CompileEqResult> {}

impl<T> CompileEq<T, TypesEq> for T {}
impl<T, U> CompileEq<U, TypesNotEq> for T {}
