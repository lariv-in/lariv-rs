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
//! | [`ListLayer`](crate::layers::ListLayer) | Paginated, filtered list query |
//! | [`CreateLayer`](crate::layers::CreateLayer) | Validate and insert a new row |
//! | [`UpdateLayer`](crate::layers::UpdateLayer) | Validate and save changes |
//! | [`DeleteLayer`](crate::layers::DeleteLayer) | Remove a loaded row |
//!
//! Auth and role layers live in the users plugin (`AuthenticationLayer`, `RoleLayer`).
//!
//! # Building a view stack
//!
//! ```ignore
//! use lariv_rs::layers::{
//!     DetailLayer, ListLayer, PathLayer, View, view, PathTag, run_layers,
//! };
//!
//! type ItemKey;
//!
//! let v = view()
//!     .layer(PathLayer::new().param("id"))
//!     .layer(DetailLayer::<ItemLoader, ItemKey>::new());
//!
//! // In handler:
//! let data = run_layers(&v, &layer_request).await?;
//! ```
//!
//! # Detail + update flow
//!
//! Load a record, then mutate it on POST:
//!
//! ```ignore
//! use lariv_rs::layers::{DetailLayer, UpdateLayer, PathLayer, view};
//!
//! type PostKey;
//!
//! let edit_view = view()
//!     .layer(PathLayer::new().param("id"))
//!     .layer(DetailLayer::<PostLoader, PostKey>::new())
//!     .layer(UpdateLayer::<PostUpdater, PostKey, PostForm>::new());
//! ```
//!
//! Place [`DetailLayer`](crate::layers::DetailLayer) immediately before
//! [`UpdateLayer`](crate::layers::UpdateLayer) or [`DeleteLayer`](crate::layers::DeleteLayer)
//! so the model sits at the HList head.
//!
//! # Plugin-specific loaders
//!
//! Layers are generic over traits you implement in the plugin:
//!
//! - [`LoadById`](crate::layers::LoadById) — fetch one row for `DetailLayer`
//! - [`LoadList`](crate::layers::LoadList) — fetch paginated rows for `ListLayer`
//! - [`CreateEntity`](crate::layers::CreateEntity) / [`UpdateEntity`](crate::layers::UpdateEntity) — persist forms
//!
//! See the blog and users plugins for complete examples.
//!
//! # Query and form patchers
//!
//! Customize list/detail queries and form parsing with patchers — see [`super::patch`].
