//! Maud UI components — fields, inputs, tables, shells, and HTMX helpers.
//!
//! # Component model
//!
//! Lariv UI is built with [`maud`] markup returned from functions and structs in
//! [`crate::components`]. There is no runtime component tree — parents pass `Markup` slots
//! to layout helpers at compile time.
//!
//! # Categories
//!
//! | Module | Examples |
//! |--------|----------|
//! | **Shells** | [`ShellScaffold`](crate::components::ShellScaffold), [`ShellTopbar`](crate::components::ShellTopbar) — page chrome |
//! | **Layout** | `container_row`, `container_column`, cards, sidebars |
//! | **Fields** | `field_text`, `field_title`, `field_markdown` — read-only display |
//! | **Inputs** | `input_text`, `input_select`, `input_foreign_key` — editable form controls |
//! | **Forms** | `form()` — wrap inputs with action URL and HTMX attrs |
//! | **Tables** | [`data_table_list`](crate::components::data_table_list) — sortable, paginated grids |
//! | **Buttons** | `button_submit`, `button_link`, `button_modal`, `button_post` |
//! | **HTMX** | [`form_hx_post_route`](crate::components::form_hx_post_route), [`hx_nav_app_layout`](crate::components::hx_nav_app_layout) |
//!
//! # Example: form with inputs
//!
//! ```ignore
//! use maud::html;
//! use lariv_rs::components::{form, input_text, field_text, FormOpts};
//!
//! fn edit_form(name: &str) -> maud::Markup {
//!     form(FormOpts {
//!         action: "/items/create",
//!         title: "Create item",
//!         ..Default::default()
//!     }, html! {
//!         (input_text("name", name, "Name"))
//!     })
//! }
//! ```
//!
//! # Example: data table
//!
//! ```ignore
//! use lariv_rs::components::{data_table_list, ObjectList, TableColumnHeader};
//!
//! data_table_list::<MyTableKey>(
//!     &object_list,
//!     &[
//!         TableColumnHeader { name: "Name", sort_key: "name", .. },
//!         TableColumnHeader { name: "Created", sort_key: "created_at", .. },
//!     ],
//!     /* row builder closure */
//! )
//! ```
//!
//! # Swap keys
//!
//! HTMX target regions use typed [`SwapKey`](crate::components::SwapKey) markers declared with
//! [`swap_key!`](crate::swap_key). Prefer `form_hx_post`, `data_table_list::<K>`, and
//! `hx_target` over raw `hx-target` strings.
//!
//! # Forms with validation
//!
//! For POST handling, pair UI inputs with [`html_form`](crate::html_form) derive macros on
//! the server — see [`HtmlForm`](crate::html_form::HtmlForm) and layer create/update flows.
