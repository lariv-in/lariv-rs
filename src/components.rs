//! Reusable Maud UI builders — fields, inputs, buttons, layouts, shells, and forms
//! for the Lariv application framework.
//!
//! Components return [`maud::Markup`] at compile time. Parents take named markup slots; there is
//! no runtime `dyn` component tree or keyed child mutation.
//!
//! # Markup builders
//!
//! Single-purpose builders render one fragment: read-only [`field::FieldText`],
//! editable [`input::InputText`], or action [`button::ButtonLink`]. Compose them
//! inside layout containers or pass them as `Markup` arguments.
//!
//! ```rust,ignore
//! use maud::html;
//! use lariv_rs::components::{field_text, FieldText, layout_card};
//!
//! let body = html! {
//!     (field_text(FieldText { value: "Hello", classes: "" }))
//! };
//! layout_card(body)
//! ```
//!
//! # Layout parents
//!
//! Containers such as [`layout::LayoutCard`], [`layout::LayoutSidebar`], and
//! [`menu::SidebarMenu`] wrap child markup. Use them for page sections, navigation
//! panels, and scaffold content slots rather than hand-rolling grid markup.
//!
//! ```rust,ignore
//! use lariv_rs::components::{layout_sidebar, LayoutSidebar, sidebar_menu, SidebarMenu};
//!
//! layout_sidebar(LayoutSidebar {
//!     sidebar: sidebar_menu(SidebarMenu { items: &[], ..Default::default() }),
//!     content: page_body,
//! })
//! ```
//!
//! # Shell
//!
//! [`shell::ShellScaffold`] (and variants) produce the root HTML document: CDN stack,
//! topbar, sidebar, and main content region. Assign a shell in route handlers so every
//! page shares chrome, alerts, and navigation.
//!
//! ```rust,ignore
//! use lariv_rs::components::{shell_scaffold, ShellScaffold};
//!
//! shell_scaffold(ShellScaffold {
//!     title: "Dashboard",
//!     sidebar: app_menu,
//!     content: dashboard_body,
//!     ..Default::default()
//! })
//! ```
//!
//! # Forms
//!
//! Presentational [`form::form`] wraps inputs for hand-built pages. For typed
//! request parsing and widget rendering, use [`crate::html_form::HtmlForm`] with
//! the `#[html_form]` attribute macro — see [`crate::html_form`].
//!
//! # HTMX swap keys
//!
//! Swappable DOM regions use compile-time [`swap::SwapKey`] types (via [`crate::swap_key!`]).
//! Prefer typed helpers (`form_hx_post`, `hx_target`, `data_table_list::<K>`, `modal_keyed`)
//! over free-form `hx-target` strings or `htmx.ajax` glue. The shell is HTMX 4 only
//! (`outerHTML` for `#app-layout` navigations, `outerMorph` for same-structure
//! fragments; swap/indicator use `:inherited`, navigation targets are explicit).
//! Alpine remains for local chrome
//! (theme, sidebar, search, table view toggle, FK display) via `hx-alpine-compat`.
//!
//! ```rust,ignore
//! use lariv_rs::components::{form_hx_post_route, hx_target, MainContentKey};
//!
//! // POST replaces only the main content region.
//! let attrs = form_hx_post_route::<MainContentKey, _>(save_route);
//! ```

pub mod attrs;
pub mod button;
pub mod container;
pub mod delete_confirmation;
pub mod detail;
pub mod field;
pub mod form;
pub mod htmx;
pub mod input;
pub mod label;
pub mod layout;
pub mod menu;
pub mod modal;
pub mod shell;
pub mod slots;
pub mod swap;
pub mod table;
pub mod text;
pub mod timeline;

#[cfg(test)]
#[cfg(all(test, feature = "plugin-dashboard", feature = "plugin-users"))]
mod smoke_tests;

pub use attrs::HtmlAttrs;
pub use button::{
    ButtonClear, ButtonDownload, ButtonLink, ButtonModal, ButtonModalForm, ButtonPost,
    ButtonSubmit, button_clear, button_download, button_download_route, button_link,
    button_link_route, button_link_url, button_modal, button_modal_form,
    button_modal_form_route, button_modal_form_urls, button_modal_route, button_post,
    button_post_route, button_submit,
};
pub use container::{container_column, container_error, container_html, container_row};
pub use delete_confirmation::{DeleteConfirmation, delete_confirmation};
pub use detail::detail;
pub use field::{
    FieldCheckbox, FieldDate, FieldDatetime, FieldDuration, FieldLink, FieldManyToMany,
    FieldMarkdown, FieldPhone, FieldSubtitle, FieldText, FieldTextarea, FieldTime, FieldTitle,
    field_checkbox, field_date, field_datetime, field_duration, field_link, field_many_to_many,
    field_markdown, field_phone, field_subtitle, field_text, field_textarea, field_time, field_title,
    render_markdown,
};
pub use form::{FormOpts, form};
pub use htmx::{
    form_get_region_route, form_post_region_route, hx_head_append, hx_partial_with_head,
    row_attr_navigate, row_attr_navigate_route, row_attr_select, row_attr_select_multi,
    HTMX_SELECT_UNSET, HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL,
};
pub use swap::{
    AppLayoutKey, MainContentKey, ModalHostKey, SwapKey, form_hx_get_route, form_hx_get_url,
    form_hx_post_main, form_hx_post_main_url, form_hx_post_route, form_hx_post_url,
    form_hx_post_selector, form_hx_boost_post_main, form_post_download, form_post_download_route,
    fragment_response, hx_nav_app_layout, hx_nav_app_layout_for_url, hx_nav_app_layout_url,
    hx_target, hx_target_swap, nav_content_attrs, nav_main_attrs, oob_attrs, oob_attrs_swap,
    oob_delete, oob_fragment, region_attrs,
};
pub use input::{
    InputCheckbox, InputDate, InputDatetime, InputEmail, InputFile, InputForeignKey,
    InputManyToMany, InputNumber, InputPassword, InputPhone, InputRadioGroup, InputRadioOption,
    InputSelect, InputSelectOption, InputText, InputTextarea, InputTime, ManyToManyItem,
    input_checkbox, input_date, input_datetime, input_email, input_file, input_foreign_key,
    input_many_to_many, input_number, input_password, input_phone, input_radio_group, input_select,
    input_text, input_textarea, input_time,
};
pub use label::{label_inline, label_inline_with_classes, label_newline};
pub use layout::{
    LayoutCard, LayoutSidebar, LayoutSimple, LayoutTopbar, layout_card, layout_main, layout_sidebar,
    layout_simple, layout_topbar,
};
pub use menu::{SidebarMenu, SidebarMenuBack, SidebarMenuItem, sidebar_menu, sidebar_menu_item};
pub use modal::{Modal, modal, modal_keyed};
pub use shell::{
    ShellAuth, ShellBase, ShellScaffold, ShellSimple, ShellTopbar, shell_auth, shell_base,
    shell_scaffold, shell_simple, shell_topbar,
};
pub use slots::{
    CoreTitle, CoreTitleTag, FoldChrome, FoldSlots, HeadSlotTag, RenderSlot, RightSidebarSlotTag,
    SharedChromeFolder, ShellChrome, SlotBucket, SlotCap, SlotCapability, SlotCtx, SlotOf,
    SlotRegistrar, SlotTag, TopbarItemsSlotTag, document_title, set_document_title, with_slots,
};
pub use table::{
    DataTable, DataTableDisplay, ObjectList, PaginationPage, TableButtonCreate, TableButtonFilter,
    TableColumnHeader, TableListContent, TablePagination, TableRow, column_sort_url, data_table,
    data_table_list, data_table_list_grid, data_table_list_grid_with_subtitle, data_table_list_opts,
    data_table_list_with_subtitle, next_sort_clause, pagination_pages, sort_indicator,
    table_button_create, table_button_filter, table_list_content, table_pagination,
};
pub use timeline::{Timeline, TimelineItem, timeline};
pub use text::{escaped_string, icon, icon_with_attrs, raw_string};
