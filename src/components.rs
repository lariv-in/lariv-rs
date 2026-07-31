//! Typed Maud UI builders (Lariv components port).
//!
//! Components return [`maud::Markup`]. Parents take named markup slots — there is
//! no runtime `dyn` component tree or keyed child mutation.
//!
//! # HTMX swap keys
//!
//! Swappable DOM regions use compile-time [`swap::SwapKey`] types (via [`crate::swap_key!`]).
//! Prefer typed helpers (`form_hx_post`, `hx_target`, `data_table_list::<K>`, `modal_keyed`)
//! over free-form `hx-target` strings or `htmx.ajax` glue. The shell is HTMX 4 only
//! (`outerHTML` for `#app-layout` navigations, `outerMorph` for same-structure
//! fragments, explicit `:inherited` boost). Alpine remains for local chrome
//! (theme, sidebar, search, table view toggle, FK display) via `hx-alpine-compat`.

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

#[cfg(test)]
mod smoke_tests;

pub use attrs::HtmlAttrs;
pub use button::{
    ButtonClear, ButtonDownload, ButtonLink, ButtonModal, ButtonModalForm, ButtonPost,
    ButtonSubmit, button_clear, button_download, button_link, button_modal, button_modal_form,
    button_post, button_submit,
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
    row_attr_navigate, row_attr_select, row_attr_select_multi, HTMX_SELECT_UNSET,
    HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL,
};
pub use swap::{
    AppLayoutKey, MainContentKey, ModalHostKey, SwapKey, form_hx_get, form_hx_post,
    form_hx_post_main, form_hx_post_selector, fragment_response, hx_target, hx_target_swap,
    nav_content_attrs, nav_main_attrs, oob_attrs, oob_attrs_swap, oob_delete, oob_fragment,
    region_attrs,
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
    CoreTitle, CoreTitleTag, FoldSlots, HeadSlotTag, RegisterSlots, RegisterSlotsHook, RenderSlot,
    RightSidebarSlotTag, ShellChrome, SlotBucket, SlotCap, SlotCapability, SlotCtx, SlotOf, SlotTag,
    TopbarItemsSlotTag, document_title, set_document_title, with_slots,
};
pub use table::{
    DataTable, DataTableDisplay, ObjectList, PaginationPage, TableButtonCreate, TableButtonFilter,
    TableColumnHeader, TableListContent, TablePagination, TableRow, column_sort_url, data_table,
    data_table_list, data_table_list_opts, next_sort_clause, pagination_pages, sort_indicator,
    table_button_create, table_button_filter, table_list_content, table_pagination,
};
pub use text::{escaped_string, icon, icon_with_attrs, raw_string};
