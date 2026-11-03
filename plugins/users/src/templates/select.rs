use maud::{Markup, html};

use lariv_core::components::{
    FieldText, FilterClauseView, ListFilterBuilder, ObjectList, PaginationPage, ShellChrome,
    SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_fk_select,
    column_sort_url, field_text, form_hx_get_picker_route, list_filter_builder, row_attr_select,
    sort_indicator, table_button_filter, table_pagination_picker,
};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::picker::{RenderPickerSelect, picker_create_button};
use lariv_core::template::RenderTemplate;

use crate::create_modals::USER_CREATE;
use crate::keys::{USER_SELECT_MODAL, USER_SELECT_TABLE};

use crate::routes::UsersSelectRouteTag;

use super::list::{UserRow, filtered_data_table};

pub struct UserSelectPage {
    pub users: ObjectList<UserRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub current_user_id: i64,
    pub current_user_name: String,
    pub page_size: u32,
}

fn render_picker_pagination(
    key: SwapKey,
    path_and_query: &str,
    number: u32,
    num_pages: u32,
) -> Markup {
    let hx_target = key.selector();
    let owned = lariv_core::components::pagination_pages(path_and_query, number, num_pages, false);
    let pages: Vec<PaginationPage<'_>> = owned
        .iter()
        .map(|(ellipsis, url, push_url, active, label)| PaginationPage {
            ellipsis: *ellipsis,
            url: url.as_str(),
            push_url: *push_url,
            active: *active,
            label: label.as_str(),
        })
        .collect();
    table_pagination_picker(TablePagination {
        pages: &pages,
        hx_target: &hx_target,
    })
}

impl RenderPickerSelect for UserSelectPage {
    fn render_table(&self) -> Markup {
        let target = if self.target_input.is_empty() {
            "UserID"
        } else {
            self.target_input.as_str()
        };
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let email_sort = column_sort_url(&self.path_and_query, "Email", &self.sort);
        let phone_sort = column_sort_url(&self.path_and_query, "Phone", &self.sort);
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let email_label = format!("Email{}", sort_indicator(&self.sort, "Email"));
        let phone_label = format!("Phone{}", sort_indicator(&self.sort, "Phone"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Email",
                label: &email_label,
                sort_url: Some(&email_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Phone",
                label: &phone_label,
                sort_url: Some(&phone_sort),
                push_url: false,
            },
        ];
        let rows: Vec<TableRow> = self
            .users
            .items
            .iter()
            .map(|u| TableRow {
                attrs: row_attr_select(target, &u.id.to_string(), &u.name),
                cells: vec![
                    field_text(FieldText {
                        value: &u.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &u.email,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &u.phone,
                        classes: "",
                    }),
                ],
                key: String::new(),
                children: Vec::new(),
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_picker_route(UsersSelectRouteTag)
                        .set("hx-push-url", "false"),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="target_input" value=(self.target_input.as_str()) {}
                    },
                }),
                ..Default::default()
            }))
            (button_fk_select(
                "Me",
                target,
                &self.current_user_id.to_string(),
                &self.current_user_name,
            ))
            (picker_create_button(USER_CREATE,
                &self.target_input,
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        let pagination = render_picker_pagination(
            USER_SELECT_MODAL,
            &self.path_and_query,
            self.users.number,
            self.users.num_pages,
        );
        filtered_data_table(
            USER_SELECT_TABLE,
            "Select User",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
            &self.filter_errors,
        )
    }
}

impl RenderTemplate for UserSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal(USER_SELECT_MODAL).into_inner()
    }
}
