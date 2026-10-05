//! Query patchers for list/detail layers.
//!
//! # Query patchers
//!
//! [`QueryPatcher`](crate::layers::QueryPatcher) modifies SeaORM selects before list or detail
//! layers execute. Use them for:
//!
//! - Eager-loading relations (avoid N+1 queries)
//! - Tenant or account scoping (`WHERE account_id = ?`)
//! - Full-text search filters from query params
//! - Custom sort orders
//!
//! ```ignore
//! use lariv_rs::layers::QueryPatcher;
//! use sea_orm::{ColumnTrait, QueryFilter, QuerySelect};
//!
//! struct PublishedOnly;
//!
//! impl QueryPatcher<BlogPostEntity> for PublishedOnly {
//!     fn patch(&self, query: sea_orm::Select<BlogPostEntity>) -> sea_orm::Select<BlogPostEntity> {
//!         query.filter(blog_post::Column::Published.eq(true))
//!     }
//! }
//!
//! // Attach to DetailLayer via query_patchers field
//! ```
//!
//! Fold multiple patchers as an HList with [`FoldQueryPatchers`](crate::layers::FoldQueryPatchers).
//!
//! # Built-in patterns
//!
//! The blog plugin demonstrates tag hierarchy filters; the users plugin applies role-based
//! query scoping on list views. Browse those plugins' `layers.rs` files for real patcher impls.
