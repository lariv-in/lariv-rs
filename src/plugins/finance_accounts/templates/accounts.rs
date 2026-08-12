use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    components::{
        ButtonClear, ButtonSubmit, Crumb, FieldLink, FieldText, FieldTitle, FormOpts,
        ObjectList, ShellChrome, TableButtonFilter, TableColumnHeader, TableRow,
        ManyToManyItem,
        ButtonModalForm, breadcrumbs, button_clear, button_delete, button_modal_form, button_submit, container_column,
        container_row, data_table_list_refresh, detail, field_link, field_text, field_title, form,
        form_hx_get_picker_route, form_hx_get_route, form_hx_post_main, form_hx_post_url,
        label_inline, modal_keyed,
        row_attr_navigate_route, table_button_filter, SwapKey,
        column_sort_url, sort_indicator,
    },
    html_form::{FormCtx, FormFieldKey, HtmlForm},
    picker::RenderPickerSelect,
    template::{RenderAppPane, RenderTemplate},
    web::modal_create_post_url,
};

use crate::plugins::finance_accounts::{account_select::{account_select_parent_up_url, account_selection_drill_attrs, account_selection_row_attrs}, account_validation::ACCOUNT_PARENT_UP_ROW_ID, entities::account, forms::{
        AccountFilterForm, AccountFilterFormField, AccountForm, AccountFormField, AccountFormFlag,
        AccountSelectionFilterForm, AccountSelectionFilterFormField,
    }, keys::{
        AccountCreateModalKey, AccountJournalEntriesTableKey, AccountJournalEntryItemsTableKey,
        AccountSelectModalKey, AccountSelectTableKey, AccountTableKey,
    }, routes::{
        AccountCreateGetRouteTag, AccountCreatePostRouteTag, AccountDeletePostRouteTag,
        AccountDetailRouteTag, AccountEditGetRouteTag, AccountEditPostRouteTag,
        AccountJournalEntriesRouteTag, AccountJournalEntryItemsRouteTag, AccountSelectRouteTag,
        FinanceDefaultRouteTag, JournalEntryDetailRouteTag,
    }};

use super::journals::JournalEntryRow;

fn account_form_inputs_with_balance_sync(balance_type: &str, inputs: Markup) -> Markup {
    let x_data = AccountForm::balance_type_sync_x_data(balance_type);
    let handler = AccountForm::balance_type_sync_fk_handler();
    html! {
        (PreEscaped(format!(
            r#"<div x-data="{}" @fk-select.window="{}">"#,
            html_escape_attr(&x_data),
            html_escape_attr(&handler),
        )))
        (inputs)
        (PreEscaped("</div>"))
    }
}

fn html_escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

use super::common::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_content, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar, layout_with_sidebar_crumbs,
    render_pagination, render_picker_pagination,
};
use crate::plugins::finance_accounts::accounting_detail_menu::{DetailMenuNavItem, detail_sidebar_menu};

fn accounts_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Accounts",
        href: None,
    }])
}

/// `ancestors` is root → … → parent (not including the current account).
fn account_crumbs(
    ancestors: &[(i64, String)],
    id: i64,
    name: &str,
    action: Option<&str>,
) -> Markup {
    let list_url = FinanceDefaultRouteTag.url();
    let detail_url = AccountDetailRouteTag::new(id).url();
    let ancestor_urls: Vec<String> = ancestors
        .iter()
        .map(|(aid, _)| AccountDetailRouteTag::new(*aid).url())
        .collect();
    let mut items: Vec<Crumb<'_>> = Vec::with_capacity(ancestors.len() + 3);
    items.push(Crumb {
        label: "Accounts",
        href: Some(&list_url),
    });
    for (i, (_, aname)) in ancestors.iter().enumerate() {
        items.push(Crumb {
            label: aname.as_str(),
            href: Some(&ancestor_urls[i]),
        });
    }
    match action {
        None => items.push(Crumb {
            label: name,
            href: None,
        }),
        Some(act) => {
            items.push(Crumb {
                label: name,
                href: Some(&detail_url),
            });
            items.push(Crumb {
                label: act,
                href: None,
            });
        }
    }
    breadcrumbs(&items)
}

fn account_detail_menu(id: i64, name: &str, active: &str, can_edit: bool) -> Markup {
    let menu_title = format!("Account: {name}");
    let detail_url = AccountDetailRouteTag::new(id).url();
    let mut nav = vec![
        DetailMenuNavItem {
            title: "Account Detail",
            url: detail_url,
            active: active == "detail",
        },
        DetailMenuNavItem {
            title: "Journal Entries",
            url: AccountJournalEntriesRouteTag::new(id).url(),
            active: active == "journal-entries",
        },
        DetailMenuNavItem {
            title: "Journal Entry Items",
            url: AccountJournalEntryItemsRouteTag::new(id).url(),
            active: active == "journal-entry-items",
        },
    ];
    if can_edit {
        nav.push(DetailMenuNavItem {
            title: "Edit Account",
            url: AccountEditGetRouteTag::new(id).url(),
            active: active == "edit",
        });
    }
    detail_sidebar_menu(
        menu_title,
        &nav,
        None,
        html! {},
    )
}

#[derive(Clone)]
pub struct AccountRow {
    pub id: i64,
    pub name: String,
    pub code: i32,
    pub is_group: bool,
    pub balance_type: String,
    pub parent_label: String,
}

fn account_filter_form(
    name: &str,
    code: &str,
    is_group: bool,
    balance_type: &str,
) -> Markup {
    let bt_choices = crate::plugins::finance_accounts::forms::balance_type_filter_choices();
    form(FormOpts {
        attrs: form_hx_get_route::<AccountTableKey, FinanceDefaultRouteTag>(
            FinanceDefaultRouteTag,
        ),
        inputs: AccountFilterForm::render_inputs(
            &FormCtx::form::<AccountFilterForm>()
                .value(AccountFilterFormField::Name, name)
                .value(AccountFilterFormField::Code, code)
                .value(AccountFilterFormField::IsGroup, if is_group { "on" } else { "" })
                .value(AccountFilterFormField::BalanceType, balance_type)
                .choices(AccountFilterFormField::BalanceType, &bt_choices),
        ),
        actions: html! {
            (container_row("flex gap-2", html! {
                (button_submit(ButtonSubmit { label: "Apply Filters", ..Default::default() }))
                (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
            }))
        },
        ..Default::default()
    })
}

fn account_children_row_attrs(row_id: i64, parent_account_id: i64) -> crate::components::attrs::HtmlAttrs {
    if row_id == ACCOUNT_PARENT_UP_ROW_ID {
        if parent_account_id > 0 {
            return row_attr_navigate_route(AccountDetailRouteTag::new(parent_account_id));
        }
        return row_attr_navigate_route(FinanceDefaultRouteTag);
    }
    row_attr_navigate_route(AccountDetailRouteTag::new(row_id))
}

fn account_create_url(parent_id: i64) -> String {
    if parent_id > 0 {
        format!("{}?ParentID={parent_id}", AccountCreateGetRouteTag.url())
    } else {
        AccountCreateGetRouteTag.url()
    }
}

fn account_row_display(row: &AccountRow) -> String {
    if row.id == ACCOUNT_PARENT_UP_ROW_ID {
        row.name.clone()
    } else {
        format!("{} — {}", row.code, row.name)
    }
}

#[derive(Generic)]
pub struct AccountListPage {
    pub accounts: ObjectList<AccountRow>,
    pub filter_name: String,
    pub filter_code: String,
    pub filter_is_group: bool,
    pub filter_balance_type: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl AccountListPage {
    pub fn render_table(&self) -> Markup {
        let code_sort = column_sort_url(&self.path_and_query, "Code", &self.sort);
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let balance_sort = column_sort_url(&self.path_and_query, "Balance", &self.sort);
        let code_label = format!("Code{}", sort_indicator(&self.sort, "Code"));
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let balance_label = format!("Balance{}", sort_indicator(&self.sort, "Balance"));
        let headers = [
            TableColumnHeader {
                key: "Code",
                label: &code_label,
                sort_url: Some(&code_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Balance",
                label: &balance_label,
                sort_url: Some(&balance_sort),
                push_url: true,
            },
            TableColumnHeader {  key: "Parent",label: "Parent", sort_url: None, push_url: true },
        ];
        let rows: Vec<TableRow> = self
            .accounts
            .items
            .iter()
            .map(|a| {
                let kind = if a.is_group { "Group" } else { "Leaf" };
                TableRow {
                    attrs: row_attr_navigate_route(AccountDetailRouteTag::new(a.id)),
                    cells: vec![
                        field_text(FieldText {
                            value: &a.code.to_string(),
                            classes: "",
                        }),
                        field_text(FieldText { value: &a.name, classes: "" }),
                        field_text(FieldText { value: kind, classes: "" }),
                        field_text(FieldText { value: &a.balance_type, classes: "" }),
                        field_text(FieldText { value: &a.parent_label, classes: "" }),
                    ],
                }
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: account_filter_form(
                    &self.filter_name,
                    &self.filter_code,
                    self.filter_is_group,
                    &self.filter_balance_type,
                ),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_accounts.AccountCreateForm",
                    href: &AccountCreateGetRouteTag.url(),
                    form_post_url: &AccountCreateGetRouteTag.path(),
                    modal_uid: AccountCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_pagination::<AccountTableKey>(
            &self.path_and_query,
            self.accounts.number,
            self.accounts.num_pages,
        );
        data_table_list_refresh::<AccountTableKey>(
            "Chart of Accounts",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for AccountListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(&self.path_and_query, accounts_list_crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(accounts_list_crumbs(), self.body())
    }
}

impl RenderTemplate for AccountListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Chart of Accounts",
            chrome,
            accounts_list_crumbs(),
            self.body(),
            &self.path_and_query,
        )
    }
}

#[derive(Generic)]
pub struct AccountDetailPage {
    pub id: i64,
    pub name: String,
    pub code: i32,
    pub is_group: bool,
    pub balance_type: String,
    pub parent_label: String,
    pub parent_id: i64,
    /// Root → … → parent (excludes this account).
    pub ancestors: Vec<(i64, String)>,
    pub balance_total: String,
    pub children: ObjectList<AccountRow>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl AccountDetailPage {
    pub fn render_children_table(&self) -> Markup {
        self.children_table()
    }

    fn children_table(&self) -> Markup {
        let code_sort = column_sort_url(&self.path_and_query, "Code", &self.sort);
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let balance_sort = column_sort_url(&self.path_and_query, "Balance", &self.sort);
        let code_label = format!("Code{}", sort_indicator(&self.sort, "Code"));
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let balance_label = format!("Balance{}", sort_indicator(&self.sort, "Balance"));
        let headers = [
            TableColumnHeader {
                key: "Code",
                label: &code_label,
                sort_url: Some(&code_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Balance",
                label: &balance_label,
                sort_url: Some(&balance_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .children
            .items
            .iter()
            .map(|a| {
                let kind = if a.is_group { "Group" } else { "Leaf" };
                let code_display = if a.id == ACCOUNT_PARENT_UP_ROW_ID {
                    "—".to_string()
                } else {
                    a.code.to_string()
                };
                let balance_display = if a.id == ACCOUNT_PARENT_UP_ROW_ID {
                    "—".to_string()
                } else {
                    a.balance_type.clone()
                };
                TableRow {
                    attrs: account_children_row_attrs(a.id, self.parent_id),
                    cells: vec![
                        field_text(FieldText {
                            value: &code_display,
                            classes: "",
                        }),
                        field_text(FieldText { value: &a.name, classes: "" }),
                        field_text(FieldText { value: kind, classes: "" }),
                        field_text(FieldText {
                            value: &balance_display,
                            classes: "",
                        }),
                    ],
                }
            })
            .collect();
        let mut actions = html! {};
        if self.can_edit {
            actions = html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_accounts.AccountCreateForm",
                    href: &account_create_url(self.id),
                    form_post_url: &AccountCreateGetRouteTag.path(),
                    modal_uid: AccountCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        data_table_list_refresh::<AccountTableKey>(
            "Sub-accounts",
            actions,
            &headers,
            &rows,
            html! {},
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        let kind = if self.is_group { "Group account" } else { "Leaf account" };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: &self.name, classes: "" }))
                    (field_text(FieldText {
                        value: &format!("Code {} · {}", self.code, kind),
                        classes: "text-base-content/70",
                    }))
                    (label_inline("Balance type", field_text(FieldText { value: &self.balance_type, classes: "" })))
                    (label_inline("Subtree balance", field_text(FieldText { value: &self.balance_total, classes: "" })))
                    @if !self.parent_label.is_empty() {
                        (label_inline("Parent", field_text(FieldText { value: &self.parent_label, classes: "" })))
                    }
                    @if self.is_group {
                        div class="mt-6" {
                            (self.children_table())
                        }
                    }
                }))
            }))
        }
    }

    fn menu(&self) -> Markup {
        account_detail_menu(self.id, &self.name, "detail", self.can_edit)
    }
}

impl RenderAppPane for AccountDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = account_crumbs(&self.ancestors, self.id, &self.name, None);
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(
            account_crumbs(&self.ancestors, self.id, &self.name, None),
            self.body(),
        )
    }
}

impl RenderTemplate for AccountDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = account_crumbs(&self.ancestors, self.id, &self.name, None);
        app_scaffold_with_sidebar("Account", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct AccountJournalEntriesPage {
    pub id: i64,
    pub name: String,
    /// Root → … → parent (excludes this account).
    pub ancestors: Vec<(i64, String)>,
    pub entries: ObjectList<JournalEntryRow>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl AccountJournalEntriesPage {
    pub fn render_entries_table(&self) -> Markup {
        self.entries_table()
    }

    fn entries_table(&self) -> Markup {
        let id_sort = column_sort_url(&self.path_and_query, "ID", &self.sort);
        let datetime_sort = column_sort_url(&self.path_and_query, "DateTime", &self.sort);
        let id_label = format!("ID{}", sort_indicator(&self.sort, "ID"));
        let datetime_label = format!("Date & time{}", sort_indicator(&self.sort, "DateTime"));
        let headers = [
            TableColumnHeader {
                key: "ID",
                label: &id_label,
                sort_url: Some(&id_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "DateTime",
                label: &datetime_label,
                sort_url: Some(&datetime_sort),
                push_url: true,
            },
            TableColumnHeader {  key: "Journal",label: "Journal", sort_url: None, push_url: false },
            TableColumnHeader {  key: "SourceDocumentType",label: "Source document type", sort_url: None, push_url: false },
            TableColumnHeader {  key: "SourceDocument",label: "Source document", sort_url: None, push_url: false },
            TableColumnHeader {  key: "Amount",label: "Amount", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self
            .entries
            .items
            .iter()
            .map(|e| {
                let instance_cell = if e.source_doc_url.is_empty() {
                    field_text(FieldText {
                        value: &e.source_doc_instance_name,
                        classes: "",
                    })
                } else {
                    field_link(FieldLink {
                        href: &e.source_doc_url,
                        label: &e.source_doc_instance_name,
                        classes: "",
                    })
                };
                TableRow {
                    attrs: row_attr_navigate_route(JournalEntryDetailRouteTag::new(e.id)),
                    cells: vec![
                        field_text(FieldText { value: &e.id.to_string(), classes: "" }),
                        field_text(FieldText { value: &e.datetime, classes: "" }),
                        field_text(FieldText { value: &e.journal_name, classes: "" }),
                        field_text(FieldText { value: &e.source_doc_label, classes: "" }),
                        instance_cell,
                        field_text(FieldText { value: &e.amount, classes: "" }),
                    ],
                }
            })
            .collect();
        let pagination = render_pagination::<AccountJournalEntriesTableKey>(
            &self.path_and_query,
            self.entries.number,
            self.entries.num_pages,
        );
        data_table_list_refresh::<AccountJournalEntriesTableKey>(
            "Journal entries",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        self.entries_table()
    }

    fn menu(&self) -> Markup {
        account_detail_menu(self.id, &self.name, "journal-entries", self.can_edit)
    }
}

impl RenderAppPane for AccountJournalEntriesPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = account_crumbs(
            &self.ancestors,
            self.id,
            &self.name,
            Some("Journal entries"),
        );
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(
            account_crumbs(
                &self.ancestors,
                self.id,
                &self.name,
                Some("Journal entries"),
            ),
            self.body(),
        )
    }
}

impl RenderTemplate for AccountJournalEntriesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = account_crumbs(
            &self.ancestors,
            self.id,
            &self.name,
            Some("Journal entries"),
        );
        app_scaffold_with_sidebar(
            "Account Journal Entries",
            chrome,
            self.menu(),
            crumbs,
            self.body(),
        )
    }
}

#[derive(Clone)]
pub struct AccountJournalEntryItemRow {
    pub datetime: String,
    pub amount: String,
    pub source_doc_instance_name: String,
    pub source_doc_url: String,
}

#[derive(Generic)]
pub struct AccountJournalEntryItemsPage {
    pub id: i64,
    pub name: String,
    /// Root → … → parent (excludes this account).
    pub ancestors: Vec<(i64, String)>,
    pub items: ObjectList<AccountJournalEntryItemRow>,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
}

impl AccountJournalEntryItemsPage {
    pub fn render_items_table(&self) -> Markup {
        self.items_table()
    }

    fn items_table(&self) -> Markup {
        let datetime_sort = column_sort_url(&self.path_and_query, "DateTime", &self.sort);
        let amount_sort = column_sort_url(&self.path_and_query, "Amount", &self.sort);
        let datetime_label = format!("Date & time{}", sort_indicator(&self.sort, "DateTime"));
        let amount_label = format!("Amount{}", sort_indicator(&self.sort, "Amount"));
        let headers = [
            TableColumnHeader {
                key: "DateTime",
                label: &datetime_label,
                sort_url: Some(&datetime_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Amount",
                label: &amount_label,
                sort_url: Some(&amount_sort),
                push_url: true,
            },
            TableColumnHeader {  key: "SourceDocument",label: "Source document", sort_url: None, push_url: false },
        ];
        let rows: Vec<TableRow> = self
            .items
            .items
            .iter()
            .map(|item| {
                let source_cell = if item.source_doc_url.is_empty() {
                    field_text(FieldText {
                        value: &item.source_doc_instance_name,
                        classes: "",
                    })
                } else {
                    field_link(FieldLink {
                        href: &item.source_doc_url,
                        label: &item.source_doc_instance_name,
                        classes: "",
                    })
                };
                TableRow {
                    attrs: crate::components::HtmlAttrs::new(),
                    cells: vec![
                        field_text(FieldText { value: &item.datetime, classes: "" }),
                        field_text(FieldText {
                            value: &item.amount,
                            classes: "tabular-nums",
                        }),
                        source_cell,
                    ],
                }
            })
            .collect();
        let pagination = render_pagination::<AccountJournalEntryItemsTableKey>(
            &self.path_and_query,
            self.items.number,
            self.items.num_pages,
        );
        data_table_list_refresh::<AccountJournalEntryItemsTableKey>(
            "Journal entry items",
            html! {},
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        self.items_table()
    }

    fn menu(&self) -> Markup {
        account_detail_menu(self.id, &self.name, "journal-entry-items", self.can_edit)
    }
}

impl RenderAppPane for AccountJournalEntryItemsPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = account_crumbs(
            &self.ancestors,
            self.id,
            &self.name,
            Some("Journal entry items"),
        );
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(
            account_crumbs(
                &self.ancestors,
                self.id,
                &self.name,
                Some("Journal entry items"),
            ),
            self.body(),
        )
    }
}

impl RenderTemplate for AccountJournalEntryItemsPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = account_crumbs(
            &self.ancestors,
            self.id,
            &self.name,
            Some("Journal entry items"),
        );
        app_scaffold_with_sidebar(
            "Account Journal Entry Items",
            chrome,
            self.menu(),
            crumbs,
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct AccountFormPage {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub is_group: bool,
    pub balance_type: String,
    pub parent_id: String,
    pub parent_display: String,
    /// Root → … → parent (excludes this account).
    pub ancestors: Vec<(i64, String)>,
    pub child_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl AccountFormPage {
    pub fn from_model(
        a: &account::Model,
        parent_display: String,
        ancestors: Vec<(i64, String)>,
        child_items: Vec<ManyToManyItem>,
    ) -> Self {
        Self {
            id: a.id,
            name: a.name.clone(),
            code: a.code.to_string(),
            is_group: a.is_group,
            balance_type: a.balance_type.to_string(),
            parent_id: a.parent_id.map(|p| p.to_string()).unwrap_or_default(),
            parent_display,
            ancestors,
            child_items,
            error: String::new(),
        }
    }

    fn body(&self) -> Markup {
        let child_picker_url = format!(
            "{}?exclude_account_id={}",
            AccountSelectRouteTag.url(),
            self.id
        );
        let bt_choices = crate::plugins::finance_accounts::forms::balance_type_choices();
        html! {
            (container_column("@container", html! {
                (field_title(FieldTitle { value: "Edit Account", classes: "" }))
                (form(FormOpts {
                    attrs: form_hx_post_main(AccountEditPostRouteTag::new(self.id)),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: account_form_inputs_with_balance_sync(
                        &self.balance_type,
                        AccountForm::render_inputs(
                            &FormCtx::form::<AccountForm>()
                                .value(AccountFormField::Name, &self.name)
                                .value(AccountFormField::Code, &self.code)
                                .value(AccountFormField::IsGroup, if self.is_group { "on" } else { "" })
                                .value(AccountFormField::BalanceType, &self.balance_type)
                                .choices(AccountFormField::BalanceType, &bt_choices)
                                .value(AccountFormField::ParentId, &self.parent_id)
                                .display(AccountFormField::ParentId, &self.parent_display)
                                .url(
                                    AccountFormField::ParentId,
                                    &format!(
                                        "{}?exclude_account_id={}",
                                        AccountSelectRouteTag.url(),
                                        self.id
                                    ),
                                )
                                .flag(AccountFormFlag::EditChildren, self.is_group)
                                .m2m(AccountFormField::ChildIds, &self.child_items)
                                .url(AccountFormField::ChildIds, &child_picker_url),
                        ),
                    ),
                    actions: html! {
                        (container_row("flex gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save Account",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                            (button_delete(
                                AccountDeletePostRouteTag::new(self.id),
                                "Delete Account",
                                "Permanently delete this account?",
                            ))
                        }))
                    },
                    ..Default::default()
                }))
            }))
        }
    }

    fn sidebar(&self) -> Markup {
        account_detail_menu(self.id, &self.name, "edit", true)
    }
}

impl RenderAppPane for AccountFormPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = account_crumbs(&self.ancestors, self.id, &self.name, Some("Edit"));
        layout_with_entity_sidebar_crumbs(self.sidebar(), crumbs, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(
            account_crumbs(&self.ancestors, self.id, &self.name, Some("Edit")),
            self.body(),
        )
    }
}

impl RenderTemplate for AccountFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = account_crumbs(&self.ancestors, self.id, &self.name, Some("Edit"));
        app_scaffold_with_sidebar(
            "Edit Account",
            chrome,
            self.sidebar(),
            crumbs,
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct AccountCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub code: String,
    pub is_group: bool,
    pub balance_type: String,
    pub parent_id: String,
    pub parent_display: String,
    pub error: String,
}

impl RenderTemplate for AccountCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_accounts.AccountCreateForm"
        } else {
            self.form_name.as_str()
        };
        let bt_choices = crate::plugins::finance_accounts::forms::balance_type_choices();
        modal_keyed::<AccountCreateModalKey>(
            "",
            form(FormOpts {
                title: "Create Account",
                subtitle: "Create a new account",
                classes: "@container",
                attrs: form_hx_post_url::<AccountCreateModalKey>(
                    &modal_create_post_url(
                        AccountCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    ),
                ),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: account_form_inputs_with_balance_sync(
                    &self.balance_type,
                    AccountForm::render_inputs(
                        &FormCtx::form::<AccountForm>()
                            .value(AccountFormField::Name, &self.name)
                            .value(AccountFormField::Code, &self.code)
                            .value(AccountFormField::IsGroup, if self.is_group { "on" } else { "" })
                            .value(AccountFormField::BalanceType, &self.balance_type)
                            .choices(AccountFormField::BalanceType, &bt_choices)
                            .value(AccountFormField::ParentId, &self.parent_id)
                            .display(AccountFormField::ParentId, &self.parent_display)
                            .url(AccountFormField::ParentId, &AccountSelectRouteTag.url()),
                    ),
                ),
                actions: html! {
                    (container_row("flex justify-end gap-2 mt-2", html! {
                        (button_submit(ButtonSubmit {
                            label: "Save Account",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                    }))
                },
                ..Default::default()
            }),
        )
    }
}

#[derive(Generic)]
pub struct AccountSelectPage {
    pub accounts: ObjectList<AccountRow>,
    pub filter_name: String,
    pub filter_code: String,
    pub filter_balance_type: String,
    pub balance_type_scope: String,
    pub parent_id: i64,
    pub grandparent_id: Option<i64>,
    pub sort: String,
    pub path_and_query: String,
    pub target_input: String,
    pub exclude_account_id: i64,
    pub can_edit: bool,
}

impl AccountSelectPage {
    fn filter_form(&self) -> Markup {
        let bt_choices = crate::plugins::finance_accounts::forms::balance_type_filter_choices();
        form(FormOpts {
            attrs: form_hx_get_picker_route::<
                AccountSelectTableKey,
                AccountSelectModalKey,
                AccountSelectRouteTag,
            >(AccountSelectRouteTag),
            inputs: html! {
                (AccountSelectionFilterForm::render_inputs(
                    &FormCtx::form::<AccountSelectionFilterForm>()
                        .value(AccountSelectionFilterFormField::Name, &self.filter_name)
                        .value(AccountSelectionFilterFormField::Code, &self.filter_code)
                        .value(AccountSelectionFilterFormField::BalanceType, &self.filter_balance_type)
                        .choices(AccountSelectionFilterFormField::BalanceType, &bt_choices)
                        .value(
                            AccountSelectionFilterFormField::ParentId,
                            if self.parent_id > 0 {
                                self.parent_id.to_string()
                            } else {
                                String::new()
                            },
                        ),
                ))
                @if !self.balance_type_scope.is_empty() {
                    input type="hidden" name="balance_type_scope" value=(self.balance_type_scope) {}
                }
                @if self.exclude_account_id > 0 {
                    input type="hidden" name="exclude_account_id" value=(self.exclude_account_id) {}
                }
                input type="hidden" name="target_input" value=(self.target_input) {}
            },
            actions: html! {
                (container_row("flex gap-2", html! {
                    (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
                }))
            },
            ..Default::default()
        })
    }

    pub fn render_table(&self) -> Markup {
        let parent_picker =
            self.target_input == AccountFormField::ParentId.target_input();
        let child_picker =
            self.target_input == AccountFormField::ChildIds.target_input();
        let show_open_column = parent_picker || child_picker;
        let code_sort = column_sort_url(&self.path_and_query, "Code", &self.sort);
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let balance_sort = column_sort_url(&self.path_and_query, "Balance", &self.sort);
        let code_label = format!("Code{}", sort_indicator(&self.sort, "Code"));
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let balance_label = format!("Balance{}", sort_indicator(&self.sort, "Balance"));
        let mut headers = vec![
            TableColumnHeader {
                key: "Code",
                label: &code_label,
                sort_url: Some(&code_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: false,
            },
            TableColumnHeader {
                key: "Balance",
                label: &balance_label,
                sort_url: Some(&balance_sort),
                push_url: false,
            },
        ];
        if show_open_column {
            headers.push(TableColumnHeader {  key: "Actions",label: "", sort_url: None, push_url: false });
        }
        let parent_up_url = account_select_parent_up_url(&self.path_and_query, self.grandparent_id);
        let rows: Vec<TableRow> = self
            .accounts
            .items
            .iter()
            .map(|a| {
                let display = account_row_display(a);
                let drill_id = if a.id == ACCOUNT_PARENT_UP_ROW_ID {
                    self.parent_id
                } else {
                    a.id
                };
                let code_str = if a.id == ACCOUNT_PARENT_UP_ROW_ID {
                    String::new()
                } else {
                    a.code.to_string()
                };
                let mut cells = vec![
                    field_text(FieldText {
                        value: &code_str,
                        classes: "",
                    }),
                    field_text(FieldText { value: &a.name, classes: "" }),
                    field_text(FieldText { value: &a.balance_type, classes: "" }),
                ];
                if show_open_column {
                    let open_cell = if a.is_group && a.id != ACCOUNT_PARENT_UP_ROW_ID {
                        let drill = account_selection_drill_attrs(&self.path_and_query, drill_id);
                        html! {
                            (PreEscaped(format!("<button{}>Open</button>", drill.as_string())))
                        }
                    } else {
                        html! {}
                    };
                    cells.push(open_cell);
                }
                TableRow {
                    attrs: account_selection_row_attrs(
                        a.id,
                        a.is_group,
                        &a.balance_type,
                        &self.target_input,
                        &display,
                        &self.path_and_query,
                        parent_up_url.as_deref(),
                        drill_id,
                    ),
                    cells,
                }
            })
            .collect();
        let create_href = account_create_url(self.parent_id);
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: self.filter_form(),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_finance_accounts.AccountCreateForm",
                    href: &create_href,
                    form_post_url: &AccountCreateGetRouteTag.path(),
                    modal_uid: AccountCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let pagination = render_picker_pagination::<AccountSelectModalKey>(
            &self.path_and_query,
            self.accounts.number,
            self.accounts.num_pages,
        );
        data_table_list_refresh::<AccountSelectTableKey>(
            "Select Account",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderPickerSelect<AccountSelectTableKey, AccountSelectModalKey> for AccountSelectPage {
    fn render_table(&self) -> Markup {
        AccountSelectPage::render_table(self)
    }
}

impl RenderAppPane for AccountSelectPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar(&self.path_and_query, self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_content(self.body())
    }
}

impl RenderTemplate for AccountSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.render_modal().into_inner()
    }
}
