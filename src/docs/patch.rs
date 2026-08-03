//! Query and form patchers for list/detail/create/update layers.
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
//! use lariv_rs::layers::{QueryPatcher, ListLayer};
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
//! // Attach to ListLayer or DetailLayer via query_patchers field
//! ```
//!
//! Fold multiple patchers as an HList with [`FoldQueryPatchers`](crate::layers::FoldQueryPatchers).
//!
//! # Form patchers
//!
//! [`FormPatcher`](crate::layers::FormPatcher) runs after form deserialization and before
//! validation/create/update. Use them to:
//!
//! - Normalize phone numbers or slugs
//! - Strip disallowed fields for non-admin users
//! - Derive computed columns from form input
//!
//! ```ignore
//! use lariv_rs::layers::FormPatcher;
//!
//! struct SlugFromTitle;
//!
//! impl FormPatcher<BlogForm> for SlugFromTitle {
//!     fn patch(&self, form: &mut BlogForm) {
//!         if form.slug.is_empty() {
//!             form.slug = slugify(&form.title);
//!         }
//!     }
//! }
//! ```
//!
//! # Built-in patterns
//!
//! The blog plugin demonstrates tag hierarchy filters; the users plugin applies role-based
//! query scoping on list views. Browse those plugins' `layers.rs` files for real patcher impls.
