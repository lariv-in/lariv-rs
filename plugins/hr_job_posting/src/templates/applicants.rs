use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldText, FilterClauseView,
    FormOpts, ListFilterBuilder, ObjectList, ShellChrome, TableButtonFilter, TableColumnHeader,
    TableRow, button_modal_form, button_submit, column_sort_url, container_column,
    delete_confirmation, detail, detail_header, field_text, form, form_hx_get_route,
    form_hx_post_selector, form_hx_post_url, label, list_filter_builder, modal, modal_keyed,
    render_pagination, row_attr_navigate, sort_indicator, table_button_filter,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::modal_create_post_url;

use lariv_plugin_hr::templates::{
    app_scaffold, data_table_with_filter_errors, hr_menu, scaffold_main, scaffold_pane,
};

use crate::crumbs::{applicant_crumbs, applicants_list_crumbs};
use crate::detail_menu::applicant_detail_menu;
use crate::forms::{ApplicantForm, ApplicantFormField, HireApplicantForm};
use crate::keys::{
    APPLICANT_CREATE_MODAL, APPLICANT_DELETE_MODAL, APPLICANT_EDIT_MODAL, APPLICANT_TABLE,
    HIRE_APPLICANT_MODAL,
};
use crate::routes::{
    ApplicantCreateGetRouteTag, ApplicantCreatePostRouteTag, ApplicantDeleteGetRouteTag,
    ApplicantDeletePostRouteTag, ApplicantEditGetRouteTag, ApplicantEditPostRouteTag,
    ApplicantListRouteTag, HireApplicantGetRouteTag, HireApplicantPostRouteTag,
};

fn detail_sidebar(menu: Markup, roles: &[String]) -> Option<Markup> {
    let gated = lariv_core::components::authorized_role(roles, menu);
    let raw = gated.into_string();
    if raw.trim().is_empty() {
        None
    } else {
        Some(maud::PreEscaped(raw))
    }
}

#[derive(Clone, Default)]
pub struct ApplicantFormValues {
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub date_of_birth: String,
    pub gender: String,
    pub address: String,
    pub remarks: String,
    pub job_form_id: String,
    pub job_form_display: String,
    pub resume_vnode_id: String,
    pub resume_display: String,
}

fn applicant_form_inputs(values: &ApplicantFormValues) -> Markup {
    let choices = ApplicantForm::gender_choices();
    ApplicantForm::render_inputs(
        &FormCtx::form::<ApplicantForm>(CsrfToken::current())
            .value(ApplicantFormField::Name, &values.name)
            .value(ApplicantFormField::Mobile, &values.mobile)
            .value(ApplicantFormField::Email, &values.email)
            .value(ApplicantFormField::DateOfBirth, &values.date_of_birth)
            .value(ApplicantFormField::Gender, &values.gender)
            .value(ApplicantFormField::Address, &values.address)
            .value(ApplicantFormField::Remarks, &values.remarks)
            .value(ApplicantFormField::JobFormId, &values.job_form_id)
            .display(ApplicantFormField::JobFormId, &values.job_form_display)
            .value(ApplicantFormField::ResumeVnodeId, &values.resume_vnode_id)
            .display(ApplicantFormField::ResumeVnodeId, &values.resume_display)
            .choices(ApplicantFormField::Gender, &choices),
    )
}

fn person_fields(name: &str, mobile: &str, email: &str) -> Markup {
    html! {
        (label("Name", field_text(FieldText { value: name, classes: "" })))
        (label("Mobile", field_text(FieldText { value: mobile, classes: "" })))
        (label("Email", field_text(FieldText { value: email, classes: "" })))
    }
}
#[derive(Clone)]
pub struct ApplicantRow {
    pub id: i64,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub detail_href: String,
}

fn render_applicants_table(
    people: &ObjectList<ApplicantRow>,
    sort: &str,
    path_and_query: &str,
    actions: Markup,
    errors: &[FilterRowError],
) -> Markup {
    let name_sort = column_sort_url(path_and_query, "Name", sort);
    let mobile_sort = column_sort_url(path_and_query, "Mobile", sort);
    let email_sort = column_sort_url(path_and_query, "Email", sort);
    let name_label = format!("Name{}", sort_indicator(sort, "Name"));
    let mobile_label = format!("Mobile{}", sort_indicator(sort, "Mobile"));
    let email_label = format!("Email{}", sort_indicator(sort, "Email"));
    let headers = [
        TableColumnHeader {
            key: "Name",
            label: &name_label,
            sort_url: Some(&name_sort),
            push_url: true,
        },
        TableColumnHeader {
            key: "Mobile",
            label: &mobile_label,
            sort_url: Some(&mobile_sort),
            push_url: true,
        },
        TableColumnHeader {
            key: "Email",
            label: &email_label,
            sort_url: Some(&email_sort),
            push_url: true,
        },
    ];
    let rows: Vec<TableRow> = people
        .items
        .iter()
        .map(|r| TableRow {
            attrs: row_attr_navigate(&r.detail_href),
            cells: vec![
                field_text(FieldText {
                    value: &r.name,
                    classes: "",
                }),
                field_text(FieldText {
                    value: &r.mobile,
                    classes: "",
                }),
                field_text(FieldText {
                    value: &r.email,
                    classes: "",
                }),
            ],
            key: String::new(),
            children: Vec::new(),
        })
        .collect();
    let pagination = render_pagination(
        APPLICANT_TABLE,
        path_and_query,
        people.number,
        people.num_pages,
    );
    data_table_with_filter_errors(
        APPLICANT_TABLE,
        "Applicants",
        actions,
        &headers,
        &rows,
        pagination,
        path_and_query,
        errors,
    )
}

#[derive(Generic)]
pub struct ApplicantListPage {
    pub people: ObjectList<ApplicantRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl ApplicantListPage {
    pub fn render_table(&self) -> Markup {
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(
                        ApplicantListRouteTag,
                    ),
                    apply_label: "Apply",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::ApplicantMutate>(),
        ) {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.ApplicantCreateForm",
                    href: &ApplicantCreateGetRouteTag.url(),
                    form_post_url: &ApplicantCreateGetRouteTag.path(),
                    modal_uid: APPLICANT_CREATE_MODAL.id(),
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        render_applicants_table(
            &self.people,
            &self.sort,
            &self.path_and_query,
            actions,
            &self.filter_errors,
        )
    }

    fn body(&self) -> Markup {
        self.render_table()
    }
}

impl RenderAppPane for ApplicantListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(hr_menu("applicants"), applicants_list_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(applicants_list_crumbs(), self.body())
    }
}

impl RenderTemplate for ApplicantListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "HR Applicants — Lariv",
            chrome,
            hr_menu("applicants"),
            applicants_list_crumbs(),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct ApplicantDetailPage {
    pub id: i64,
    pub display_name: String,
    pub values: ApplicantFormValues,
    pub job_form_href: String,
    pub answers: Vec<crate::questions::RenderedAnswer>,
    pub resume_href: String,
}

impl ApplicantDetailPage {
    fn body(&self) -> Markup {
        let actions = if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<crate::routes::ApplicantMutate>(),
        ) {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.HireApplicantForm",
                    href: &HireApplicantGetRouteTag::new(self.id).url(),
                    form_post_url: &HireApplicantGetRouteTag::new(self.id).path(),
                    modal_uid: HIRE_APPLICANT_MODAL.id(),
                    label: "Hire as employee",
                    classes: "btn-primary",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.ApplicantEditForm",
                    href: &ApplicantEditGetRouteTag::new(self.id).url(),
                    form_post_url: &ApplicantEditPostRouteTag::new(self.id).path(),
                    modal_uid: APPLICANT_EDIT_MODAL.id(),
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        let gender_label = lariv_plugin_hr::gender::ApplicantGender::parse(&self.values.gender)
            .map(|g| g.label().to_string())
            .unwrap_or_else(|| self.values.gender.clone());
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (person_fields(&self.values.name, &self.values.mobile, &self.values.email))
                    @if !self.values.date_of_birth.is_empty() {
                        (label("Date of birth", field_text(FieldText { value: &self.values.date_of_birth, classes: "" })))
                    }
                    @if !self.values.gender.is_empty() {
                        (label("Gender", field_text(FieldText { value: &gender_label, classes: "" })))
                    }
                    @if !self.values.address.is_empty() {
                        (label("Address", field_text(FieldText { value: &self.values.address, classes: "" })))
                    }
                    @if !self.values.remarks.is_empty() {
                        (label("Remarks", field_text(FieldText { value: &self.values.remarks, classes: "" })))
                    }
                    @if !self.values.job_form_display.is_empty() {
                        (label("Job posting", html! {
                            @if self.job_form_href.is_empty() {
                                (field_text(FieldText { value: &self.values.job_form_display, classes: "" }))
                            } @else {
                                a class="link link-primary" href=(self.job_form_href) {
                                    (self.values.job_form_display)
                                }
                            }
                        }))
                    }
                    @if !self.answers.is_empty() {
                        (label("Application answers", html! {
                            dl class="grid gap-3" {
                                @for answer in &self.answers {
                                    div {
                                        dt class="font-medium" { (answer.display_text) }
                                        @if !answer.description.is_empty() {
                                            dd class="text-sm opacity-70" { (answer.description) }
                                        }
                                        dd class="text-sm opacity-70" { (answer.type_label) }
                                        dd { (answer.answer) }
                                    }
                                }
                            }
                        }))
                    }
                    @if !self.values.resume_display.is_empty() {
                        (label("Resume", html! {
                            @if self.resume_href.is_empty() {
                                (field_text(FieldText { value: &self.values.resume_display, classes: "" }))
                            } @else {
                                a class="link link-primary" href=(self.resume_href) {
                                    (self.values.resume_display)
                                }
                            }
                        }))
                    }
                }))
            }))
        }
    }
}

impl RenderAppPane for ApplicantDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            detail_sidebar(
                applicant_detail_menu(&self.display_name, self.id, "detail"),
                &lariv_plugin_users::role_authorization::roles_for::<crate::routes::ApplicantMutate>(
                ),
            ),
            applicant_crumbs(&self.display_name),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(applicant_crumbs(&self.display_name), self.body())
    }
}

impl RenderTemplate for ApplicantDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Applicant — Lariv",
            chrome,
            detail_sidebar(
                applicant_detail_menu(&self.display_name, self.id, "detail"),
                &lariv_plugin_users::role_authorization::roles_for::<crate::routes::ApplicantMutate>(
                ),
            ),
            applicant_crumbs(&self.display_name),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct ApplicantCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub values: ApplicantFormValues,
    pub error: String,
}

impl RenderTemplate for ApplicantCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            APPLICANT_CREATE_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New applicant" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(APPLICANT_CREATE_MODAL, &modal_create_post_url(
                        ApplicantCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: applicant_form_inputs(&self.values),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create applicant", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct ApplicantEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub values: ApplicantFormValues,
    pub error: String,
}

impl RenderTemplate for ApplicantEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = ApplicantDeleteGetRouteTag::new(self.id).url();
        modal_keyed(
            APPLICANT_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit applicant" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(APPLICANT_EDIT_MODAL, &self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: applicant_form_inputs(&self.values),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_hr.ApplicantDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: APPLICANT_DELETE_MODAL.id(),
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
pub struct HireApplicantModalPage {
    pub applicant_id: i64,
    pub form_name: String,
    pub refresh_table: String,
    pub error: String,
}

impl RenderTemplate for HireApplicantModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            HIRE_APPLICANT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Hire applicant" }
                p class="mb-4 text-sm opacity-80" {
                    "Move this applicant into employees?"
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(HIRE_APPLICANT_MODAL, &modal_create_post_url(
                        HireApplicantPostRouteTag::new(self.applicant_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: HireApplicantForm::render_inputs(
                        &FormCtx::form::<HireApplicantForm>(CsrfToken::current()),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Hire", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct ApplicantDeleteModalPage {
    pub id: i64,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for ApplicantDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", APPLICANT_DELETE_MODAL.id());
        let post_url = ApplicantDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid: APPLICANT_DELETE_MODAL.id(),
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
