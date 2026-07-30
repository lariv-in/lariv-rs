use std::marker::PhantomData;

pub struct Tagged<T, V> {
    pub value: V,
    tag: PhantomData<T>,
}

impl<T, V> Tagged<T, V> {
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

/// Witness that two types are the same / different (see [`CompileEq`]).
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
pub trait CompileEq<U, Result: CompileEqResult> {}

impl<T> CompileEq<T, TypesEq> for T {}
impl<T, U> CompileEq<U, TypesNotEq> for T {}
