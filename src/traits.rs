//! HList manipulation traits for the capability system.
//!
//! These traits provide compile-time lookup, insertion, replacement, and removal
//! of capabilities and tagged items in frunk HLists.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`add`] | Prepend capabilities; prove tag absence |
//! | [`get`] | Borrow capabilities and tagged values by tag |
//! | [`replace`] | Map/replace capabilities in place |
//! | [`remove`] | Pluck capabilities from the HList |

pub mod add;
pub mod get;
pub mod remove;
pub mod replace;
