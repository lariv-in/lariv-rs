//! Declarative plugin HTTP route registration — proc-macro generated tags and [`RouteRegistrar`].
//!
//! ```ignore
//! define_plugin_routes! {
//!     plugin: FilesystemTag;
//!     proof: FilesystemRoutesProof;
//!     pages: [
//!         pane ListIdx, ListP => VNodeListPageTag, VNodeListPage;
//!     ];
//!     routes: [
//!         get VNodeDeleteGetRouteTag, "/filesystem/{id}/delete", handlers::nodes::delete_get, modal;
//!         get VNodeListRouteTag, "/filesystem", handlers::nodes::list, fragment(VNodeTableKey);
//!         get BlogBySlugRouteTag, "/blog/{slug}", handlers::..., param slug: String;
//!     ]
//! }
//! ```
//!
//! - `pane` pages require [`RenderAppPane`]; `page` pages are template-only.
//! - Handlers default to `handler::<Templates, Slots, _, _>`; prefix with `bare` for a raw fn.
//! - Non-bare routes default to app-pane GET/POST; add `, fragment(SwapKey)` for table filters.
//! - Bare routes **must** specify a response kind: `file`, `modal`, `redirect`, `generation`,
//!   `raw`, or `fragment(SwapKey)`.
//! - Optional `param name: Ty` overrides path param types (default: `{*x}` → `Vec<String>`,
//!   `{id}` / `{*_id}` → `i64`, else `String`).

pub use lariv_rs_macros::define_plugin_routes;
