//! View middleware layers — compile-time request pipelines.
//!
//! # What layers do
//!
//! Layers are typed middleware steps that run before a page renders or a form submits.
//! Each layer contributes data to an HList "Data" stack that handlers and templates read.
//!
//! Common built-in layers in [`crate::layers`]:
//!
//! | Layer | Purpose |
//! |-------|---------|
//! | [`PathLayer`](crate::layers::PathLayer) | Parse `{id}` / `{slug}` path params |
//! | [`MethodLayer`](crate::layers::MethodLayer) | Reject wrong HTTP methods early |
//! | [`DetailLayer`](crate::layers::DetailLayer) | Load one DB row by path id |
//! | [`DeleteLayer`](crate::layers::DeleteLayer) | Remove a loaded row |
//!
//! Auth and role layers live in the users plugin (`AuthenticationLayer`, `RoleLayer`).
//!
//! # Building a view stack
//!
//! ```ignore
//! use lariv_rs::layers::{
//!     DetailLayer, PathLayer, View, view, PathTag, run_layers,
//! };
//!
//! type ItemKey;
//!
//! let v = view()
//!     .layer(PathLayer::names(&["id"]))
//!     .layer(DetailLayer::<ItemLoader, ItemKey>::new());
//!
//! // In handler:
//! let data = run_layers(&v, &layer_request).await?;
//! ```
//!
//! # Detail + delete flow
//!
//! Load a record, then mutate or remove it on POST:
//!
//! ```ignore
//! use lariv_rs::layers::{DetailLayer, DeleteLayer, PathLayer, view};
//!
//! type PostKey;
//!
//! let edit_view = view()
//!     .layer(PathLayer::names(&["id"]))
//!     .layer(DetailLayer::<PostLoader, PostKey>::new())
//!     .layer(DeleteLayer::<PostDeleter, PostKey>::new());
//! ```
//!
//! Place [`DetailLayer`](crate::layers::DetailLayer) immediately before
//! [`DeleteLayer`](crate::layers::DeleteLayer) so the model sits at the HList head.
//!
//! # Plugin-specific loaders
//!
//! Layers are generic over traits you implement in the plugin:
//!
//! - [`LoadById`](crate::layers::LoadById) — fetch one row for `DetailLayer`
//!
//! See the filesystem and users plugins for complete examples.
//!
//! # Query patchers
//!
//! Customize list/detail queries with patchers — see [`super::patch`].
