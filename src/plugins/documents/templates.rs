use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    components::{
        ButtonClear, ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, FieldText,
        FieldTitle, FormOpts, LayoutMain, LayoutSidebar, ObjectList, PaginationPage, ShellChrome,
        ShellScaffold, SidebarMenu, SidebarMenuItem, SidebarNavLink, SlotCapability, SlotRegistrar,
        SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, breadcrumbs,
        button_clear, button_modal_form, button_submit, column_sort_url, container_column,
        container_row, data_table_list_refresh, delete_confirmation, detail, field_text,
        field_title, form, form_hx_get_route, form_hx_post_main, form_hx_post_selector,
        form_hx_post_url, label, layout_main, layout_sidebar, modal, modal_keyed, pagination_pages,
        row_attr_navigate_route, shell_scaffold, sidebar_menu, sidebar_menu_item_pane,
        sidebar_nav_items_pane, sort_indicator, table_button_filter, table_create_button,
        table_pagination, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{modal_create_post_query, modal_edit_post_url},
};

use super::forms::{
    DocumentFilterForm, DocumentFilterFormField, DocumentForm, DocumentFormField, PreferencesForm,
    PreferencesFormField,
};
use super::keys::{
    DocumentCreateModalKey, DocumentDeleteModalKey, DocumentEditModalKey, DocumentTableKey,
};
use super::logic::TypeFields;
use super::routes::{
    DocumentCreatePostRouteTag, DocumentDefaultRouteTag, DocumentDeleteGetRouteTag,
    DocumentDeletePostRouteTag, DocumentDetailRouteTag, DocumentEditGetRouteTag,
    DocumentEditPostRouteTag, DocumentPrefsGetRouteTag, DocumentPrefsPostRouteTag,
};
use crate::plugins::filesystem::routes::VNodeDetailRouteTag;

crate::define_register_items! {
    plugin: DocumentsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        DocumentListIdx: DocumentListPageTag => DocumentListPage,
        DocumentDetailIdx: DocumentDetailPageTag => DocumentDetailPage,
        DocumentEditModalIdx: DocumentEditModalPageTag => DocumentEditModalPage,
        DocumentCreateModalIdx: DocumentCreateModalPageTag => DocumentCreateModalPage,
        ConfirmDeleteIdx: DocumentConfirmDeletePageTag => ConfirmDeletePage,
        PreferencesIdx: DocumentPreferencesPageTag => DocumentPreferencesPage,
    ]
}

crate::define_register_items! {
    plugin: DocumentsTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

fn app_scaffold(
    title: &str,
    chrome: &ShellChrome,
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> Markup {
    shell_scaffold(ShellScaffold {
        title,
        registry_head: chrome.head.clone(),
        topbar_items: chrome.topbar_items.clone(),
        right_sidebar: chrome.right_sidebar.clone(),
        sidebar,
        breadcrumbs: crumbs,
        body,
        ..Default::default()
    })
}

fn scaffold_pane(
    sidebar: Markup,
    crumbs: Markup,
    body: Markup,
) -> crate::components::AppLayoutHtml {
    layout_sidebar(LayoutSidebar {
        sidebar,
        breadcrumbs: crumbs,
        content: body,
    })
}

fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

fn documents_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Documents",
        href: None,
    }])
}

fn document_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    let list_url = DocumentDefaultRouteTag.url();
    let detail_url = DocumentDetailRouteTag::new(id).url();
    let label = if name.is_empty() { "Document" } else { name };
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Documents",
                href: Some(&list_url),
            },
            Crumb { label, href: None },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Documents",
                href: Some(&list_url),
            },
            Crumb {
                label,
                href: Some(&detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

fn document_menu(current_path: &str) -> Markup {
    let list_url = DocumentDefaultRouteTag.url();
    let prefs_url = DocumentPrefsGetRouteTag.url();
    let links = [
        SidebarNavLink {
            key: "documents",
            title: "All Documents",
            url: &list_url,
            icon_name: None,
            match_prefixes: &[],
        },
        SidebarNavLink {
            key: "preferences",
            title: "Preferences",
            url: &prefs_url,
            icon_name: None,
            match_prefixes: &[],
        },
    ];
    sidebar_menu(SidebarMenu {
        title: "Documents",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

fn document_detail_menu(id: i64, name: &str) -> Markup {
    let title = if name.is_empty() {
        "Document".to_string()
    } else {
        format!("Document: {name}")
    };
    let detail_url = DocumentDetailRouteTag::new(id).url();
    let prefs_url = DocumentPrefsGetRouteTag.url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Document Detail",
                url: &detail_url,
                active: true,
                ..Default::default()
            }))
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Preferences",
                url: &prefs_url,
                ..Default::default()
            }))
        },
    })
}

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

fn fk_value(id: i64) -> String {
    if id <= 0 {
        String::new()
    } else {
        id.to_string()
    }
}

fn document_filter_form(name: &str, page_size: u32) -> Markup {
    form(
        &CsrfToken::current(),
        FormOpts {
            attrs: form_hx_get_route::<DocumentTableKey, DocumentDefaultRouteTag>(
                DocumentDefaultRouteTag,
            ),
            inputs: with_list_filter_common(
                DocumentFilterForm::render_inputs(
                    &FormCtx::form::<DocumentFilterForm>(CsrfToken::current())
                        .value(DocumentFilterFormField::Name, name),
                ),
                page_size,
            ),
            actions: html! {
                (container_row("flex gap-2", html! {
                    (button_submit(ButtonSubmit { label: "Apply Filters", ..Default::default() }))
                    (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
                }))
            },
            ..Default::default()
        },
    )
}

fn render_pagination<K: SwapKey>(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, true);
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
    table_pagination(TablePagination {
        pages: &pages,
        hx_target: K::SELECTOR,
    })
}

fn document_form_inputs(document_type: &str, fields: &TypeFields) -> Markup {
    let type_choices = choice_pairs(DocumentForm::document_type_choices());
    let gender_choices = choice_pairs(DocumentForm::gender_choices());
    let vnode_id = fk_value(fields.vnode_id);
    DocumentForm::render_inputs(
        &FormCtx::form::<DocumentForm>(CsrfToken::current())
            .value(DocumentFormField::DocumentType, document_type)
            .value(DocumentFormField::VnodeId, vnode_id.as_str())
            .display(DocumentFormField::VnodeId, &fields.vnode_name)
            .value(DocumentFormField::AadharNumber, &fields.aadhar_number)
            .value(DocumentFormField::Name, &fields.name)
            .value(DocumentFormField::Gender, &fields.gender)
            .value(DocumentFormField::DateOfBirth, &fields.date_of_birth)
            .value(DocumentFormField::Address, &fields.address)
            .choices(DocumentFormField::DocumentType, &type_choices)
            .choices(DocumentFormField::Gender, &gender_choices),
    )
}

#[derive(Clone)]
pub struct DocumentRow {
    pub id: i64,
    pub document_type: String,
    pub name: String,
    pub aadhar_number: String,
}

#[derive(Generic)]
pub struct DocumentListPage {
    pub documents: ObjectList<DocumentRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl DocumentListPage {
    pub fn render_table(&self) -> Markup {
        let type_sort = column_sort_url(&self.path_and_query, "Type", &self.sort);
        let type_label = format!("Type{}", sort_indicator(&self.sort, "Type"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: "Name",
                sort_url: None,
                push_url: true,
            },
            TableColumnHeader {
                key: "Type",
                label: &type_label,
                sort_url: Some(&type_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "AadharNumber",
                label: "Aadhar number",
                sort_url: None,
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .documents
            .items
            .iter()
            .map(|doc| TableRow {
                attrs: row_attr_navigate_route(DocumentDetailRouteTag::new(doc.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &doc.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &doc.document_type,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &doc.aadhar_number,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: document_filter_form(&self.filter_name, self.page_size),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (table_create_button::<DocumentTableKey, DocumentCreateModalKey>(
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            };
        }
        let pagination = render_pagination::<DocumentTableKey>(
            &self.path_and_query,
            self.documents.number,
            self.documents.num_pages,
        );
        data_table_list_refresh::<DocumentTableKey>(
            "Documents",
            actions,
            &headers,
            &rows,
            pagination,
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for DocumentListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            document_menu(&self.path_and_query),
            documents_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(documents_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for DocumentListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Documents — Lariv",
            chrome,
            document_menu(&self.path_and_query),
            documents_list_crumbs(),
            self.render_table(),
        )
    }
}

#[derive(Generic)]
pub struct DocumentDetailPage {
    pub id: i64,
    pub document_type: String,
    pub name: String,
    pub vnode_id: i64,
    pub vnode_name: String,
    pub aadhar_number: String,
    pub gender: String,
    pub date_of_birth: String,
    pub address: String,
    pub error: String,
    pub can_edit: bool,
    pub extra_actions: String,
}

impl DocumentDetailPage {
    fn title(&self) -> &str {
        if self.name.is_empty() {
            "Document"
        } else {
            &self.name
        }
    }

    pub(crate) fn body(&self) -> Markup {
        let file_url = VNodeDetailRouteTag::new(self.vnode_id).url();
        let file = if self.vnode_id > 0 {
            html! { a class="link" href=(file_url) { (self.vnode_name) } }
        } else {
            field_text(FieldText {
                value: &self.vnode_name,
                classes: "",
            })
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (field_title(FieldTitle { value: self.title(), classes: "" }))
                    @if !self.error.is_empty() {
                        div class="alert alert-error" { (self.error) }
                    }
                    (label("Document type", field_text(FieldText { value: &self.document_type, classes: "" })))
                    (label("Aadhar card file", file))
                    (label("Aadhar number", field_text(FieldText { value: &self.aadhar_number, classes: "" })))
                    (label("Name", field_text(FieldText { value: &self.name, classes: "" })))
                    (label("Gender", field_text(FieldText { value: &self.gender, classes: "" })))
                    (label("Date of birth", field_text(FieldText { value: &self.date_of_birth, classes: "" })))
                    (label("Address", field_text(FieldText { value: &self.address, classes: "" })))
                    @if self.can_edit && self.error.is_empty() {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                name: "p_documents.DocumentEditForm",
                                href: &DocumentEditGetRouteTag::new(self.id).url(),
                                form_post_url: &DocumentEditPostRouteTag::new(self.id).path(),
                                modal_uid: DocumentEditModalKey::ID,
                                label: "Edit",
                                classes: "btn-outline",
                                ..Default::default()
                            }))
                            (PreEscaped(self.extra_actions.clone()))
                        }))
                    } @else if self.can_edit {
                        (container_row("flex gap-2 mt-4", html! {
                            (button_modal_form(ButtonModalForm {
                                label: "Delete",
                                icon_name: Some("trash"),
                                name: "p_documents.DocumentDeleteForm",
                                href: &DocumentDeleteGetRouteTag::new(self.id).url(),
                                form_post_url: &DocumentDeletePostRouteTag::new(self.id).path(),
                                modal_uid: DocumentDeleteModalKey::ID,
                                classes: "btn-error",
                                ..Default::default()
                            }))
                            (PreEscaped(self.extra_actions.clone()))
                        }))
                    } @else if !self.extra_actions.is_empty() {
                        (container_row("flex gap-2 mt-4", html! {
                            (PreEscaped(self.extra_actions.clone()))
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for DocumentDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        let crumbs = document_crumbs(self.id, &self.name, None);
        scaffold_pane(
            document_detail_menu(self.id, &self.name),
            crumbs,
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(document_crumbs(self.id, &self.name, None), self.body())
    }
}

impl RenderTemplate for DocumentDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = document_crumbs(self.id, &self.name, None);
        app_scaffold(
            "Document — Lariv",
            chrome,
            document_detail_menu(self.id, &self.name),
            crumbs,
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct DocumentEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub document_type: String,
    pub fields: TypeFields,
    pub error: String,
}

impl RenderTemplate for DocumentEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = DocumentDeleteGetRouteTag::new(self.id).url();
        modal_keyed::<DocumentEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit document" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<DocumentEditModalKey>(&modal_edit_post_url(
                        DocumentEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: document_form_inputs(&self.document_type, &self.fields),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_documents.DocumentDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: DocumentDeleteModalKey::ID,
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct DocumentCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub document_type: String,
    pub fields: TypeFields,
    pub error: String,
}

impl RenderTemplate for DocumentCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_documents.DocumentCreateForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed::<DocumentCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Add document",
                    subtitle: "Save an identity document",
                    classes: "@container",
                    attrs: form_hx_post_url::<DocumentCreateModalKey>(&modal_create_post_query(
                        DocumentCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: document_form_inputs(&self.document_type, &self.fields),
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save document",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", DocumentDeleteModalKey::ID)
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DocumentDeleteModalKey::ID
        } else {
            self.modal_uid.as_str()
        };
        let post_url = DocumentDeletePostRouteTag::new(self.id).url();
        modal(crate::components::Modal {
            uid,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

fn preferences_crumbs() -> Markup {
    let list_url = DocumentDefaultRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Documents",
            href: Some(&list_url),
        },
        Crumb {
            label: "Preferences",
            href: None,
        },
    ])
}

#[derive(Generic)]
pub struct DocumentPreferencesPage {
    pub signing_authority_name: String,
    pub validity_duration: String,
    pub error: String,
}

impl DocumentPreferencesPage {
    fn body(&self) -> Markup {
        form(
            &CsrfToken::current(),
            FormOpts {
                attrs: form_hx_post_main(DocumentPrefsPostRouteTag),
                title: "Documents Preferences",
                subtitle: "Used as the name and lifetime of the certificate that signs a PDF",
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs: PreferencesForm::render_inputs(
                    &FormCtx::form::<PreferencesForm>(CsrfToken::current())
                        .value(
                            PreferencesFormField::SigningAuthorityName,
                            self.signing_authority_name.as_str(),
                        )
                        .value(
                            PreferencesFormField::ValidityDuration,
                            self.validity_duration.as_str(),
                        ),
                ),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Save Preferences",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for DocumentPreferencesPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            document_menu(&DocumentPrefsGetRouteTag.url()),
            preferences_crumbs(),
            self.body(),
        )
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for DocumentPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Documents Preferences — Lariv",
            chrome,
            document_menu(&DocumentPrefsGetRouteTag.url()),
            preferences_crumbs(),
            self.body(),
        )
    }
}
