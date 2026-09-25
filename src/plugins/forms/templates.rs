use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use crate::{
    components::{
        ButtonModalForm, ButtonSubmit, Crumb, DeleteConfirmation, DetailHeader, FieldText,
        FormOpts, LayoutMain, LayoutSidebar, ObjectList, PaginationPage, ShellChrome,
        ShellScaffold, ShellSimple, SidebarMenu, SidebarNavLink, SlotCapability, SlotRegistrar,
        SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, breadcrumbs,
        button_modal_form, button_submit, column_sort_url, data_table_list_refresh,
        delete_confirmation, detail, detail_header, field_text, form, form_hx_get_route,
        form_hx_post_selector, form_hx_post_url, label, layout_main, layout_sidebar, modal,
        modal_keyed, pagination_pages, row_attr_navigate_route, row_attr_select, shell_scaffold,
        shell_simple, sidebar_menu, sidebar_nav_items_pane, sort_indicator, table_button_filter,
        table_create_button, table_pagination, table_pagination_picker, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, FormCtxBuilder, HtmlForm},
    http::{ProvideRequestCaps, RouteQueryBuilder},
    picker::{RenderPickerSelect, picker_create_button},
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::{
        CreateModal, modal_create_href_for_table, modal_create_post_query, modal_create_post_url,
        modal_edit_post_url,
    },
};

use super::forms::{
    FormResponseForm, FormResponseFormField, FormResponseScopedFilterForm,
    FormResponseScopedFilterFormField, PublicFormResponseForm, PublicFormResponseFormField,
    SurveyFilterForm, SurveyFilterFormField, SurveyForm, SurveyFormField,
};
use super::keys::{
    FormCreateModalKey, FormDeleteModalKey, FormDetailResponsesTableKey, FormEditModalKey,
    FormResponseCreateModalKey, FormResponseDeleteModalKey, FormResponseEditModalKey,
    FormResponseSelectModalKey, FormResponseSelectTableKey, FormSelectModalKey, FormSelectTableKey,
    FormTableKey,
};
use super::logic::questions::question_type_label;
use super::routes::{
    FormCreatePostRouteTag, FormDeleteGetRouteTag, FormDeletePostRouteTag, FormDetailRouteTag,
    FormEditGetRouteTag, FormEditPostRouteTag, FormListRouteTag, FormPublicPostRouteTag,
    FormResponseCreateGetRouteTag, FormResponseCreatePostRouteTag, FormResponseDeleteGetRouteTag,
    FormResponseDeletePostRouteTag, FormResponseDetailRouteTag, FormResponseEditGetRouteTag,
    FormResponseEditPostRouteTag,
};
use super::types::FormQuestions;

crate::define_register_items! {
    plugin: FormsTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        FormListIdx: FormListPageTag => FormListPage,
        FormDetailIdx: FormDetailPageTag => FormDetailPage,
        FormEditModalIdx: FormEditModalPageTag => FormEditModalPage,
        FormCreateModalIdx: FormCreateModalPageTag => FormCreateModalPage,
        FormSelectIdx: FormSelectPageTag => FormSelectPage,
        FormResponseSelectIdx: FormResponseSelectPageTag => FormResponseSelectPage,
        FormResponseDetailIdx: FormResponseDetailPageTag => FormResponseDetailPage,
        FormResponseEditModalIdx: FormResponseEditModalPageTag => FormResponseEditModalPage,
        FormResponseCreateModalIdx: FormResponseCreateModalPageTag => FormResponseCreateModalPage,
        ConfirmDeleteIdx: FormsConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

crate::define_register_items! {
    plugin: FormsTag;
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

fn forms_menu(current: &str) -> Markup {
    let forms_url = FormListRouteTag.url();
    let links = [SidebarNavLink {
        key: "forms",
        title: "Forms",
        url: &forms_url,
        icon_name: None,
        match_prefixes: &["/forms/"],
    }];
    sidebar_menu(SidebarMenu {
        title: "Forms",
        children: sidebar_nav_items_pane(&links, current),
    })
}

fn forms_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Forms",
        href: None,
    }])
}

fn form_detail_crumbs(_id: i64, title: &str) -> Markup {
    let list_url = FormListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Forms",
            href: Some(&list_url),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}

fn response_detail_crumbs(id: i64, form_id: i64, form_title: &str) -> Markup {
    let forms_url = FormListRouteTag.url();
    let form_url = FormDetailRouteTag::new(form_id).url();
    breadcrumbs(&[
        Crumb {
            label: "Forms",
            href: Some(&forms_url),
        },
        Crumb {
            label: form_title,
            href: Some(&form_url),
        },
        Crumb {
            label: &format!("Response #{id}"),
            href: None,
        },
    ])
}

fn form_response_create_url(form_id: i64) -> String {
    RouteQueryBuilder::new(FormResponseCreateGetRouteTag)
        .query("FormId", form_id)
        .build()
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

fn render_picker_pagination<K: SwapKey>(
    path_and_query: &str,
    number: u32,
    num_pages: u32,
) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, false);
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
        hx_target: K::SELECTOR,
    })
}

#[derive(Clone)]
pub struct FormRow {
    pub id: i64,
    pub title: String,
    pub question_count: usize,
    pub author: String,
    pub updated_at: String,
}

#[derive(Generic)]
pub struct FormListPage {
    pub forms: ObjectList<FormRow>,
    pub filter_title: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl FormListPage {
    pub fn render_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let updated_sort = column_sort_url(&self.path_and_query, "UpdatedAt", &self.sort);
        let title_label = format!("Title{}", sort_indicator(&self.sort, "Title"));
        let updated_label = format!("Updated{}", sort_indicator(&self.sort, "UpdatedAt"));
        let headers = [
            TableColumnHeader {
                key: "Title",
                label: &title_label,
                sort_url: Some(&title_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Questions",
                label: "Questions",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Author",
                label: "Author",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "UpdatedAt",
                label: &updated_label,
                sort_url: Some(&updated_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .forms
            .items
            .iter()
            .map(|f| TableRow {
                attrs: row_attr_navigate_route(FormDetailRouteTag::new(f.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &f.title,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &f.question_count.to_string(),
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &f.author,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &f.updated_at,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<FormTableKey, FormListRouteTag>(FormListRouteTag),
                    inputs: with_list_filter_common(
                        SurveyFilterForm::render_inputs(
                            &FormCtx::form::<SurveyFilterForm>(CsrfToken::current())
                                .value(SurveyFilterFormField::Title, &self.filter_title),
                        ),
                        self.page_size,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
            (table_create_button::<FormTableKey, FormCreateModalKey>(
                Some("plus"),
                "btn-square btn-outline btn-sm",
            ))
        };
        data_table_list_refresh::<FormTableKey>(
            "Forms",
            actions,
            &headers,
            &rows,
            render_pagination::<FormTableKey>(
                &self.path_and_query,
                self.forms.number,
                self.forms.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderAppPane for FormListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            forms_menu("/forms"),
            forms_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(forms_list_crumbs(), self.render_table())
    }
}

impl RenderTemplate for FormListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Forms — Lariv",
            chrome,
            forms_menu("/forms"),
            forms_list_crumbs(),
            self.render_table(),
        )
    }
}

fn color_swatch(hex: &str) -> Markup {
    html! {
        span class="inline-flex items-center gap-2" {
            span class="w-4 h-4 rounded-full shrink-0 border border-base-300" style=(format!("background-color: {hex}")) {}
            span class="font-mono text-sm" { (hex) }
        }
    }
}

#[derive(Generic)]
pub struct FormDetailPage {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
    pub uid: String,
    pub access_status: String,
    pub accent_color_hex: String,
    pub background_image: String,
    pub public_url: String,
    pub questions: FormQuestions,
    pub responses: ObjectList<FormResponseRow>,
    pub filter_name: String,
    pub filter_email: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl FormDetailPage {
    fn questions_markup(&self) -> Markup {
        if self.questions.is_empty() {
            return html! { p class="text-sm opacity-60" { "No questions defined." } };
        }
        html! {
            ol class="list-decimal list-inside flex flex-col gap-2" {
                @for q in self.questions.iter() {
                    li {
                        span class="font-medium" { (q.display_text) }
                        span class="text-xs opacity-60 ml-2" {
                            "(" (question_type_label(&q.question_type)) ")"
                        }
                        @if q.required {
                            span class="text-error text-xs ml-1" { "*" }
                        }
                        @if let Some(desc) = &q.description {
                            @if !desc.is_empty() {
                                p class="text-sm opacity-70 ml-4" { (desc) }
                            }
                        }
                    }
                }
            }
        }
    }

    fn actions(&self) -> Markup {
        let edit_get = FormEditGetRouteTag::new(self.id).url();
        let edit_post = FormEditPostRouteTag::new(self.id).url();
        html! {
            a href=(self.public_url) class="btn btn-outline" target="_blank" { "Public form" }
            (button_modal_form(ButtonModalForm {
                name: "p_forms.FormEditForm",
                href: &edit_get,
                form_post_url: &edit_post,
                modal_uid: FormEditModalKey::ID,
                label: "Edit",
                classes: "btn-outline",
                ..Default::default()
            }))
        }
    }

    pub fn render_responses_table(&self) -> Markup {
        let submitted_sort = column_sort_url(&self.path_and_query, "SubmittedAt", &self.sort);
        let name_sort = column_sort_url(&self.path_and_query, "Name", &self.sort);
        let submitted_label = format!("Submitted{}", sort_indicator(&self.sort, "SubmittedAt"));
        let name_label = format!("Name{}", sort_indicator(&self.sort, "Name"));
        let headers = [
            TableColumnHeader {
                key: "Name",
                label: &name_label,
                sort_url: Some(&name_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Email",
                label: "Email",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "SubmittedAt",
                label: &submitted_label,
                sort_url: Some(&submitted_sort),
                push_url: true,
            },
        ];
        let rows: Vec<TableRow> = self
            .responses
            .items
            .iter()
            .map(|r| TableRow {
                attrs: row_attr_navigate_route(FormResponseDetailRouteTag::new(r.id)),
                cells: vec![
                    field_text(FieldText {
                        value: &r.name,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &r.email,
                        classes: "",
                    }),
                    field_text(FieldText {
                        value: &r.submitted_at,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let create_href = modal_create_href_for_table::<FormDetailResponsesTableKey>(
            &form_response_create_url(self.id),
            FormResponseCreateModalKey::FORM_NAME,
        );
        let actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<FormDetailResponsesTableKey, FormDetailRouteTag>(
                        FormDetailRouteTag::new(self.id),
                    ),
                    inputs: with_list_filter_common(
                        FormResponseScopedFilterForm::render_inputs(
                            &FormCtx::form::<FormResponseScopedFilterForm>(CsrfToken::current())
                                .value(FormResponseScopedFilterFormField::Name, &self.filter_name)
                                .value(FormResponseScopedFilterFormField::Email, &self.filter_email),
                        ),
                        self.page_size,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
            (button_modal_form(ButtonModalForm {
                name: "",
                href: &create_href,
                form_post_url: "",
                modal_uid: FormResponseCreateModalKey::ID,
                icon_name: Some("plus"),
                classes: "btn-square btn-outline btn-sm",
                ..Default::default()
            }))
        };
        data_table_list_refresh::<FormDetailResponsesTableKey>(
            "Responses",
            actions,
            &headers,
            &rows,
            render_pagination::<FormDetailResponsesTableKey>(
                &self.path_and_query,
                self.responses.number,
                self.responses.num_pages,
            ),
            &self.path_and_query,
        )
    }

    fn pane_body(&self) -> Markup {
        html! {
            (detail(html! {
                (detail_header(DetailHeader {
                    title: &self.title,
                    actions: self.actions(),
                }))
                (label("Author", field_text(FieldText {
                    value: &self.author,
                    classes: "",
                })))
                (label("Created", field_text(FieldText {
                    value: &self.created_at,
                    classes: "",
                })))
                (label("Updated", field_text(FieldText {
                    value: &self.updated_at,
                    classes: "",
                })))
                (label("Uid", field_text(FieldText {
                    value: &self.uid,
                    classes: "",
                })))
                (label("Access", field_text(FieldText {
                    value: &self.access_status,
                    classes: "",
                })))
                (label("Description", field_text(FieldText {
                    value: &self.description,
                    classes: "whitespace-pre-wrap",
                })))
                (label("Accent color", color_swatch(&self.accent_color_hex)))
                (label("Background image", field_text(FieldText {
                    value: if self.background_image.is_empty() {
                        "—"
                    } else {
                        &self.background_image
                    },
                    classes: "",
                })))
                (label("Public link", html! {
                    a href=(self.public_url) class="link link-primary break-all" target="_blank" {
                        (self.public_url)
                    }
                }))
                div class="mt-4" {
                    h4 class="font-semibold mb-2" { "Questions" }
                    (self.questions_markup())
                }
            }))
            div class="mt-6" {
                (self.render_responses_table())
            }
        }
    }
}

impl RenderAppPane for FormDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            forms_menu("/forms"),
            form_detail_crumbs(self.id, &self.title),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(form_detail_crumbs(self.id, &self.title), self.pane_body())
    }
}

impl RenderTemplate for FormDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("{} — Forms", self.title),
            chrome,
            forms_menu("/forms"),
            form_detail_crumbs(self.id, &self.title),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct FormEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub title: String,
    pub description: String,
    pub access_status: String,
    pub accent_color: String,
    pub background_vnode_id: String,
    pub background_display: String,
    pub questions_json: String,
    pub created_by_id: i64,
    pub author_display: String,
    pub error: String,
}

impl RenderTemplate for FormEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = FormDeleteGetRouteTag::new(self.id).url();
        let created_by_id_s = fk_value(self.created_by_id);
        let access_choices = access_status_choice_pairs();
        let ctx = survey_form_ctx(
            &self.title,
            &self.description,
            &self.access_status,
            &self.accent_color,
            &self.background_vnode_id,
            &self.background_display,
            &self.questions_json,
            &created_by_id_s,
            &self.author_display,
            &access_choices,
        );
        modal_keyed::<FormEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit form" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<FormEditModalKey>(&modal_edit_post_url(
                        FormEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: SurveyForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_forms.FormDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: FormDeleteModalKey::ID,
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
pub struct FormCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub target_input: String,
    pub title: String,
    pub description: String,
    pub access_status: String,
    pub accent_color: String,
    pub background_vnode_id: String,
    pub background_display: String,
    pub questions_json: String,
    pub created_by_id: i64,
    pub author_display: String,
    pub error: String,
}

impl RenderTemplate for FormCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_forms.FormCreateForm"
        } else {
            self.form_name.as_str()
        };
        let created_by_id_s = fk_value(self.created_by_id);
        let access_choices = access_status_choice_pairs();
        let ctx = survey_form_ctx(
            &self.title,
            &self.description,
            &self.access_status,
            &self.accent_color,
            &self.background_vnode_id,
            &self.background_display,
            &self.questions_json,
            &created_by_id_s,
            &self.author_display,
            &access_choices,
        );
        modal_keyed::<FormCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create form",
                    attrs: form_hx_post_url::<FormCreateModalKey>(&modal_create_post_query(
                        FormCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                        &self.target_input,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: SurveyForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Save",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

/// Accent + optional background used by unauthenticated public form pages.
pub struct PublicFormLook {
    pub accent_color_hex: String,
    pub accent_content_hex: String,
    pub background_url: Option<String>,
}

fn render_public_document(title: &str, look: &PublicFormLook, body: Markup) -> Markup {
    let mut css = format!(
        "html,body,[data-theme]{{--color-accent:{} !important;--color-accent-content:{} !important;}}",
        look.accent_color_hex, look.accent_content_hex
    );
    if let Some(url) = &look.background_url {
        css.push_str(&format!(
            "html,body{{background-image:url(\"{url}\");background-size:cover;background-position:center;background-attachment:fixed;}}"
        ));
    }
    let extra_head = html! {
        title { (title) }
        style { (PreEscaped(css)) }
    };
    shell_simple(ShellSimple {
        title,
        extra_head,
        body,
        ..Default::default()
    })
}

fn public_card(inner: Markup) -> Markup {
    html! {
        main class="container mx-auto max-w-3xl px-4 py-10" {
            div class="card bg-base-100/90 shadow-xl" {
                div class="card-body" {
                    (inner)
                }
            }
        }
    }
}

pub struct PublicFormPage {
    pub uid: String,
    pub title: String,
    pub description: String,
    pub look: PublicFormLook,
    pub name: String,
    pub email: String,
    pub answers_json: String,
    pub questions_json: String,
    pub error: String,
}

impl PublicFormPage {
    pub fn render(&self) -> Markup {
        let post_url = FormPublicPostRouteTag::new(self.uid.clone()).url();
        let mut ctx: FormCtx = FormCtx::form::<PublicFormResponseForm>(CsrfToken::current())
            .value(PublicFormResponseFormField::Name, self.name.as_str())
            .value(PublicFormResponseFormField::Email, self.email.as_str())
            .value(
                PublicFormResponseFormField::AnswersJson,
                self.answers_json.as_str(),
            )
            .into();
        ctx = ctx.set_display("questions_json", self.questions_json.as_str());
        ctx = ctx.set_display("forms_catalog_json", "{}");
        let body = public_card(html! {
            h1 class="text-3xl font-bold" { (self.title) }
            @if !self.description.trim().is_empty() {
                p class="opacity-80 whitespace-pre-wrap" { (self.description) }
            }
            (form(&CsrfToken::current(), FormOpts {
                action: Some(&post_url),
                method: "post",
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: PublicFormResponseForm::render_inputs(&ctx),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Submit",
                        classes: "btn-accent",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            }))
        });
        render_public_document(&self.title, &self.look, body)
    }
}

pub struct PublicFormClosedPage {
    pub title: String,
    pub look: PublicFormLook,
}

impl PublicFormClosedPage {
    pub fn render(&self) -> Markup {
        let body = public_card(html! {
            h1 class="text-3xl font-bold" { (self.title) }
            p class="opacity-80" {
                "This form is not accepting any more responses."
            }
        });
        render_public_document(&self.title, &self.look, body)
    }
}

pub struct PublicFormThanksPage {
    pub title: String,
    pub look: PublicFormLook,
}

impl PublicFormThanksPage {
    pub fn render(&self) -> Markup {
        let body = public_card(html! {
            h1 class="text-3xl font-bold" { "Response submitted" }
            p class="opacity-80" {
                "Thank you. Your response to " strong { (self.title) } " has been recorded."
            }
        });
        render_public_document("Response submitted", &self.look, body)
    }
}

#[derive(Clone)]
pub struct FormOption {
    pub id: i64,
    pub title: String,
}

#[derive(Generic)]
pub struct FormSelectPage {
    pub forms: ObjectList<FormOption>,
    pub filter_title: String,
    pub target_input: String,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect<FormSelectTableKey, FormSelectModalKey> for FormSelectPage {
    fn render_table(&self) -> Markup {
        let rows: Vec<TableRow> = self
            .forms
            .items
            .iter()
            .map(|f| TableRow {
                attrs: row_attr_select(&self.target_input, &f.id.to_string(), &f.title),
                cells: vec![field_text(FieldText {
                    value: &f.title,
                    classes: "",
                })],
            })
            .collect();
        data_table_list_refresh::<FormSelectTableKey>(
            "Select form",
            html! {
                (picker_create_button::<FormCreateModalKey>(
                    &self.target_input,
                    Some("plus"),
                    "btn-square btn-outline btn-sm",
                ))
            },
            &[TableColumnHeader {
                key: "Title",
                label: "Title",
                sort_url: None,
                push_url: false,
            }],
            &rows,
            render_picker_pagination::<FormSelectTableKey>(
                &self.path_and_query,
                self.forms.number,
                self.forms.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for FormSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<FormSelectModalKey>("", self.render_table())
    }
}

#[derive(Clone)]
pub struct FormResponseOption {
    pub id: i64,
    pub label: String,
}

#[derive(Generic)]
pub struct FormResponseSelectPage {
    pub responses: ObjectList<FormResponseOption>,
    pub filter_name: String,
    pub target_input: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl RenderPickerSelect<FormResponseSelectTableKey, FormResponseSelectModalKey>
    for FormResponseSelectPage
{
    fn render_table(&self) -> Markup {
        let rows: Vec<TableRow> = self
            .responses
            .items
            .iter()
            .map(|r| TableRow {
                attrs: row_attr_select(&self.target_input, &r.id.to_string(), &r.label),
                cells: vec![field_text(FieldText {
                    value: &r.label,
                    classes: "",
                })],
            })
            .collect();
        data_table_list_refresh::<FormResponseSelectTableKey>(
            "Select form response",
            html! {},
            &[TableColumnHeader {
                key: "Name",
                label: "Response",
                sort_url: None,
                push_url: false,
            }],
            &rows,
            render_picker_pagination::<FormResponseSelectTableKey>(
                &self.path_and_query,
                self.responses.number,
                self.responses.num_pages,
            ),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for FormResponseSelectPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<FormResponseSelectModalKey>("", self.render_table())
    }
}

#[derive(Clone)]
pub struct FormResponseRow {
    pub id: i64,
    pub form_id: i64,
    pub form_title: String,
    pub name: String,
    pub email: String,
    pub submitted_at: String,
}

#[derive(Clone)]
pub struct AnswerDisplayRow {
    pub question: String,
    pub answer: String,
}

#[derive(Generic)]
pub struct FormResponseDetailPage {
    pub id: i64,
    pub form_id: i64,
    pub form_title: String,
    pub name: String,
    pub email: String,
    pub submitted_at: String,
    pub answers: Vec<AnswerDisplayRow>,
}

impl FormResponseDetailPage {
    fn title(&self) -> String {
        format!("Response #{}", self.id)
    }

    fn answers_markup(&self) -> Markup {
        if self.answers.is_empty() {
            return html! { p class="text-sm opacity-60" { "No answers recorded." } };
        }
        html! {
            dl class="flex flex-col gap-2" {
                @for row in &self.answers {
                    div {
                        dt class="font-medium text-sm" { (row.question) }
                        dd class="text-sm opacity-80 ml-2" { (row.answer) }
                    }
                }
            }
        }
    }

    fn actions(&self) -> Markup {
        let edit_get = FormResponseEditGetRouteTag::new(self.id).url();
        let edit_post = FormResponseEditPostRouteTag::new(self.id).url();
        html! {
            (button_modal_form(ButtonModalForm {
                name: "p_forms.FormResponseEditForm",
                href: &edit_get,
                form_post_url: &edit_post,
                modal_uid: FormResponseEditModalKey::ID,
                label: "Edit",
                classes: "btn-outline",
                ..Default::default()
            }))
        }
    }

    fn pane_body(&self) -> Markup {
        let form_url = FormDetailRouteTag::new(self.form_id).url();
        let title = self.title();
        detail(html! {
            (detail_header(DetailHeader {
                title: &title,
                actions: self.actions(),
            }))
            (label("Form", html! {
                a href=(form_url) class="link" { (self.form_title) }
            }))
            (label("Name", field_text(FieldText {
                value: &self.name,
                classes: "",
            })))
            (label("Email", field_text(FieldText {
                value: &self.email,
                classes: "",
            })))
            (label("Submitted", field_text(FieldText {
                value: &self.submitted_at,
                classes: "",
            })))
            div class="mt-4" {
                h4 class="font-semibold mb-2" { "Answers" }
                (self.answers_markup())
            }
        })
    }
}

impl RenderAppPane for FormResponseDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            forms_menu("/forms"),
            response_detail_crumbs(self.id, self.form_id, &self.form_title),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(
            response_detail_crumbs(self.id, self.form_id, &self.form_title),
            self.pane_body(),
        )
    }
}

impl RenderTemplate for FormResponseDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("Response #{} — Forms", self.id),
            chrome,
            forms_menu("/forms"),
            response_detail_crumbs(self.id, self.form_id, &self.form_title),
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct FormResponseEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub form_id: i64,
    pub form_display: String,
    pub name: String,
    pub email: String,
    pub submitted_at: String,
    pub answers_json: String,
    pub questions_json: String,
    pub forms_catalog_json: String,
    pub error: String,
}

impl RenderTemplate for FormResponseEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = FormResponseDeleteGetRouteTag::new(self.id).url();
        let ctx = response_form_ctx(self);
        modal_keyed::<FormResponseEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit response" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<FormResponseEditModalKey>(&modal_edit_post_url(
                        FormResponseEditPostRouteTag::new(self.id),
                        &self.form_name,
                    ))
                        .set("hx-swap", "outerHTML"),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: FormResponseForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_forms.FormResponseDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: FormResponseDeleteModalKey::ID,
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
pub struct FormResponseCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub form_id: i64,
    pub form_display: String,
    pub name: String,
    pub email: String,
    pub submitted_at: String,
    pub answers_json: String,
    pub questions_json: String,
    pub forms_catalog_json: String,
    pub error: String,
}

impl RenderTemplate for FormResponseCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_forms.FormResponseCreateForm"
        } else {
            self.form_name.as_str()
        };
        let ctx = response_form_ctx(self);
        modal_keyed::<FormResponseCreateModalKey>(
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create response",
                    attrs: form_hx_post_url::<FormResponseCreateModalKey>(&modal_create_post_url(
                        FormResponseCreatePostRouteTag,
                        form_name,
                        &self.refresh_table,
                    ))
                    .set("hx-swap", "outerHTML"),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: FormResponseForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Save",
                            classes: "btn-primary",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}

trait ResponseModalFields {
    fn form_id(&self) -> i64;
    fn form_display(&self) -> &str;
    fn name(&self) -> &str;
    fn email(&self) -> &str;
    fn submitted_at(&self) -> &str;
    fn answers_json(&self) -> &str;
    fn questions_json(&self) -> &str;
    fn forms_catalog_json(&self) -> &str;
}

impl ResponseModalFields for FormResponseCreateModalPage {
    fn form_id(&self) -> i64 {
        self.form_id
    }
    fn form_display(&self) -> &str {
        &self.form_display
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn email(&self) -> &str {
        &self.email
    }
    fn submitted_at(&self) -> &str {
        &self.submitted_at
    }
    fn answers_json(&self) -> &str {
        &self.answers_json
    }
    fn questions_json(&self) -> &str {
        &self.questions_json
    }
    fn forms_catalog_json(&self) -> &str {
        &self.forms_catalog_json
    }
}

impl ResponseModalFields for FormResponseEditModalPage {
    fn form_id(&self) -> i64 {
        self.form_id
    }
    fn form_display(&self) -> &str {
        &self.form_display
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn email(&self) -> &str {
        &self.email
    }
    fn submitted_at(&self) -> &str {
        &self.submitted_at
    }
    fn answers_json(&self) -> &str {
        &self.answers_json
    }
    fn questions_json(&self) -> &str {
        &self.questions_json
    }
    fn forms_catalog_json(&self) -> &str {
        &self.forms_catalog_json
    }
}

fn response_form_ctx<T: ResponseModalFields>(page: &T) -> FormCtx<'_> {
    let form_id_s = fk_value(page.form_id());
    let mut ctx: FormCtx = FormCtx::form::<FormResponseForm>(CsrfToken::current())
        .value(FormResponseFormField::FormId, form_id_s)
        .display(FormResponseFormField::FormId, page.form_display())
        .value(FormResponseFormField::Name, page.name().to_string())
        .value(FormResponseFormField::Email, page.email().to_string())
        .value(
            FormResponseFormField::SubmittedAt,
            page.submitted_at().to_string(),
        )
        .value(
            FormResponseFormField::AnswersJson,
            page.answers_json().to_string(),
        )
        .into();
    ctx = ctx.set_display("questions_json", page.questions_json());
    ctx = ctx.set_display("forms_catalog_json", page.forms_catalog_json());
    ctx
}

fn fk_value(id: i64) -> String {
    if id <= 0 {
        String::new()
    } else {
        id.to_string()
    }
}

fn survey_form_ctx<'a>(
    title: &'a str,
    description: &'a str,
    access_status: &'a str,
    accent_color: &'a str,
    background_vnode_id: &'a str,
    background_display: &'a str,
    questions_json: &'a str,
    created_by_id: &'a str,
    author_display: &'a str,
    access_choices: &'a [(String, String)],
) -> FormCtxBuilder<'a, SurveyForm> {
    FormCtx::form::<SurveyForm>(CsrfToken::current())
        .value(SurveyFormField::Title, title)
        .value(SurveyFormField::Description, description)
        .value(SurveyFormField::AccessStatus, access_status)
        .value(SurveyFormField::AccentColor, accent_color)
        .value(SurveyFormField::BackgroundVnodeId, background_vnode_id)
        .display(SurveyFormField::BackgroundVnodeId, background_display)
        .value(SurveyFormField::QuestionsJson, questions_json)
        .value(SurveyFormField::CreatedById, created_by_id)
        .display(SurveyFormField::CreatedById, author_display)
        .choices(SurveyFormField::AccessStatus, access_choices)
}

fn access_status_choice_pairs() -> Vec<(String, String)> {
    SurveyForm::access_status_choices()
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
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
        let target = format!("#{}", self.modal_uid);
        let post_url = if self.modal_uid == FormResponseDeleteModalKey::ID {
            FormResponseDeletePostRouteTag::new(self.id).url()
        } else {
            FormDeletePostRouteTag::new(self.id).url()
        };
        modal(crate::components::Modal {
            uid: self.modal_uid.as_str(),
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn select_page(target_input: &str) -> FormSelectPage {
        FormSelectPage {
            forms: ObjectList::from_page(vec![], 1, 12, 0),
            filter_title: String::new(),
            target_input: target_input.into(),
            sort: String::new(),
            path_and_query: "/forms/pick".into(),
            page_size: 12,
        }
    }

    #[test]
    fn select_table_create_button_embeds_target_input() {
        let html = select_page("form_id").render_table().into_string();
        assert!(html.contains("/forms/create/"), "select: {html}");
        assert!(
            html.contains("name=p_forms.FormCreateForm"),
            "select: {html}"
        );
        assert!(html.contains("target_input=form_id"), "select: {html}");
        assert!(html.contains("fk-modal-host"), "select: {html}");
    }

    #[test]
    fn create_modal_post_embeds_target_input() {
        let html = FormCreateModalPage {
            form_name: "p_forms.FormCreateForm".into(),
            refresh_table: String::new(),
            target_input: "form_id".into(),
            title: String::new(),
            description: String::new(),
            access_status: String::new(),
            accent_color: "#6366f1".into(),
            background_vnode_id: String::new(),
            background_display: String::new(),
            questions_json: "[]".into(),
            created_by_id: 1,
            author_display: "Ada".into(),
            error: String::new(),
        }
        .render(&Default::default())
        .into_string();
        assert!(html.contains("target_input=form_id"), "create: {html}");
        assert!(html.contains("form-create-modal"), "create: {html}");
        assert!(html.contains("Description"), "create: {html}");
        assert!(html.contains("Accent color"), "create: {html}");
        assert!(html.contains("Background image"), "create: {html}");
        assert!(html.contains("type=\"color\""), "create: {html}");
        assert!(html.contains("<textarea"), "create: {html}");
    }

    #[test]
    fn public_form_uses_accent_color() {
        let html = PublicFormPage {
            uid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
            title: "Survey".into(),
            description: "Hello".into(),
            look: PublicFormLook {
                accent_color_hex: "#22c55e".into(),
                accent_content_hex: "#111827".into(),
                background_url: None,
            },
            name: String::new(),
            email: String::new(),
            answers_json: "{}".into(),
            questions_json: "[]".into(),
            error: String::new(),
        }
        .render()
        .into_string();
        assert!(
            html.contains("html,body,[data-theme]{--color-accent:#22c55e !important;"),
            "{html}"
        );
        assert!(
            html.contains("--color-accent-content:#111827 !important;"),
            "{html}"
        );
        assert!(html.contains("btn-accent"), "{html}");
        assert!(!html.contains("--color-primary:"), "{html}");
    }
}
