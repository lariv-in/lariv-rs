use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FormOpts, ObjectList,
        PaginationPage, ShellChrome, SwapKey, TableButtonFilter, TableColumnHeader,
        TablePagination, TableRow, button_modal_form, button_submit, column_sort_url,
        container_row, data_table_list_refresh, delete_confirmation, detail, detail_header, form,
        form_hx_get_route, form_hx_post_selector, form_hx_post_url, modal, modal_keyed,
        pagination_pages, row_attr_navigate, shell_scaffold, sort_indicator, table_button_filter,
        table_pagination, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    template::{RenderAppPane, RenderTemplate},
    web::modal_create_post_url,
};

use crate::plugins::hr::{
    crumbs::job_forms_crumbs,
    forms::{JobFormForm, JobFormFormField},
    keys::{JobFormCreateModalKey, JobFormDeleteModalKey, JobFormEditModalKey, JobFormTableKey},
    routes::{
        JobApplicationPublicPostRouteTag, JobFormCreateGetRouteTag, JobFormCreatePostRouteTag,
        JobFormDeleteGetRouteTag, JobFormDeletePostRouteTag, JobFormEditGetRouteTag,
        JobFormEditPostRouteTag, JobFormListRouteTag,
    },
    templates::{hr_menu, scaffold_main, scaffold_pane},
};

fn fk_value(id: i64) -> String {
    if id > 0 {
        id.to_string()
    } else {
        String::new()
    }
}

fn empty_job_form() -> JobFormForm {
    JobFormForm {
        job_title: String::new(),
        salary_range: String::new(),
        experience_required: String::new(),
        description: String::new(),
        form_id: 0,
        csrf: CsrfToken::current(),
    }
}

use crate::plugins::forms::components::{InputFormAnswers, input_form_answers};

#[derive(Clone)]
pub struct JobFormRow {
    pub id: i64,
    pub job_title: String,
    pub form_title: String,
    pub apply_href: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct JobFormListPage {
    pub rows: ObjectList<JobFormRow>,
    pub filter_title: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl JobFormListPage {
    fn render_table(&self) -> Markup {
        let title_sort = column_sort_url(&self.path_and_query, "Title", &self.sort);
        let title_label = format!("Job title{}", sort_indicator(&self.sort, "Title"));
        let headers = [
            TableColumnHeader {
                key: "Title",
                label: &title_label,
                sort_url: Some(&title_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "Form",
                label: "Application form",
                sort_url: None,
                push_url: false,
            },
            TableColumnHeader {
                key: "Apply",
                label: "Public apply link",
                sort_url: None,
                push_url: false,
            },
        ];
        let table_rows: Vec<TableRow> = self
            .rows
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row_attr_navigate(&row.detail_href),
                cells: vec![
                    html! { (row.job_title) },
                    html! { (row.form_title) },
                    html! {
                        a href=(row.apply_href) class="link link-primary" target="_blank" { "Apply" }
                    },
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<JobFormTableKey, JobFormListRouteTag>(JobFormListRouteTag),
                    inputs: with_list_filter_common(
                        html! {
                            label class="form-control w-full" {
                                span class="label-text" { "Job title" }
                                input
                                    class="input input-bordered"
                                    type="text"
                                    name="Title"
                                    value=(self.filter_title);
                            }
                        },
                        self.page_size,
                    ),
                    actions: html! {
                        (container_row("flex gap-2", html! {
                            (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                        }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if self.can_edit {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.JobFormCreateForm",
                    href: &JobFormCreateGetRouteTag.url(),
                    form_post_url: &JobFormCreateGetRouteTag.path(),
                    modal_uid: JobFormCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let owned = pagination_pages(
            &self.path_and_query,
            self.rows.number,
            self.rows.num_pages,
            true,
        );
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
        data_table_list_refresh::<JobFormTableKey>(
            "Job postings",
            actions,
            &headers,
            &table_rows,
            table_pagination(TablePagination {
                pages: &pages,
                hx_target: JobFormTableKey::SELECTOR,
            }),
            &self.path_and_query,
        )
    }
}

impl RenderTemplate for JobFormListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let body = self.render_table();
        shell_scaffold(crate::components::ShellScaffold {
            title: "Job postings",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar: hr_menu("job-forms"),
            breadcrumbs: job_forms_crumbs("Job postings"),
            body,
            ..Default::default()
        })
    }
}

impl RenderAppPane for JobFormListPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("job-forms"),
            job_forms_crumbs("Job postings"),
            self.render_table(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(job_forms_crumbs("Job postings"), self.render_table())
    }
}

#[derive(Generic)]
pub struct JobFormDetailPage {
    pub id: i64,
    pub job_title: String,
    pub salary_range: String,
    pub experience_required: String,
    pub description: String,
    pub form_title: String,
    pub apply_href: String,
    pub can_edit: bool,
}

impl JobFormDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                a href=(self.apply_href) class="btn btn-primary btn-sm" target="_blank" { "Public apply page" }
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.JobFormEditForm",
                    href: &JobFormEditGetRouteTag::new(self.id).url(),
                    form_post_url: &JobFormEditPostRouteTag::new(self.id).path(),
                    modal_uid: JobFormEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.JobFormDeleteForm",
                    href: &JobFormDeleteGetRouteTag::new(self.id).url(),
                    form_post_url: &JobFormDeleteGetRouteTag::new(self.id).path(),
                    modal_uid: JobFormDeleteModalKey::ID,
                    label: "Delete",
                    classes: "btn-outline btn-error btn-sm",
                    ..Default::default()
                }))
            }
        } else {
            html! {
                a href=(self.apply_href) class="btn btn-primary btn-sm" target="_blank" { "Public apply page" }
            }
        };
        html! {
            (detail_header(DetailHeader {
                title: &self.job_title,
                actions,
            }))
            (detail(html! {
                dl class="grid gap-3" {
                    dt { "Application form" }
                    dd { (self.form_title) }
                    @if !self.salary_range.is_empty() {
                        dt { "Salary range" }
                        dd { (self.salary_range) }
                    }
                    @if !self.experience_required.is_empty() {
                        dt { "Experience required" }
                        dd { (self.experience_required) }
                    }
                    dt { "Description" }
                    dd class="whitespace-pre-wrap" { (self.description) }
                    dt { "Apply URL" }
                    dd {
                        a href=(self.apply_href) class="link link-primary" { (self.apply_href) }
                    }
                }
            }))
        }
    }
}

impl RenderTemplate for JobFormDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_scaffold(crate::components::ShellScaffold {
            title: &self.job_title,
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar: hr_menu("job-forms"),
            breadcrumbs: job_forms_crumbs(&self.job_title),
            body: self.body(),
            ..Default::default()
        })
    }
}

impl RenderAppPane for JobFormDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("job-forms"),
            job_forms_crumbs(&self.job_title),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(job_forms_crumbs(&self.job_title), self.body())
    }
}

pub struct JobFormCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub error: String,
    pub form: JobFormForm,
}

impl JobFormCreateModalPage {
    pub fn new(form_name: String, refresh_table: String) -> Self {
        Self {
            form_name,
            refresh_table,
            error: String::new(),
            form: empty_job_form(),
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        form: &JobFormForm,
        error: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            error,
            form: JobFormForm {
                job_title: form.job_title.clone(),
                salary_range: form.salary_range.clone(),
                experience_required: form.experience_required.clone(),
                description: form.description.clone(),
                form_id: form.form_id,
                csrf: CsrfToken::current(),
            },
        }
    }
}

impl RenderTemplate for JobFormCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<JobFormForm>(CsrfToken::current())
            .value(JobFormFormField::JobTitle, &self.form.job_title)
            .value(JobFormFormField::SalaryRange, &self.form.salary_range)
            .value(
                JobFormFormField::ExperienceRequired,
                &self.form.experience_required,
            )
            .value(JobFormFormField::Description, &self.form.description)
            .value(JobFormFormField::FormId, fk_value(self.form.form_id));
        modal_keyed::<JobFormCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New job posting" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<JobFormCreateModalKey>(&modal_create_post_url(
                        JobFormCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: JobFormForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct JobFormEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub job_title: String,
    pub salary_range: String,
    pub experience_required: String,
    pub description: String,
    pub form_id: i64,
    pub form_display: String,
    pub error: String,
}

impl RenderTemplate for JobFormEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let ctx = FormCtx::form::<JobFormForm>(CsrfToken::current())
            .value(JobFormFormField::JobTitle, &self.job_title)
            .value(JobFormFormField::SalaryRange, &self.salary_range)
            .value(
                JobFormFormField::ExperienceRequired,
                &self.experience_required,
            )
            .value(JobFormFormField::Description, &self.description)
            .value(JobFormFormField::FormId, fk_value(self.form_id))
            .display(JobFormFormField::FormId, &self.form_display);
        modal_keyed::<JobFormEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit job posting" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<JobFormEditModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: JobFormForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct JobFormDeleteModalPage {
    pub id: i64,
    pub form_name: String,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for JobFormDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", JobFormDeleteModalKey::ID);
        let post_url = JobFormDeletePostRouteTag::new(self.id).url();
        modal(crate::components::Modal {
            uid: JobFormDeleteModalKey::ID,
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

pub struct JobApplicationPage {
    pub job_form_id: i64,
    pub job_title: String,
    pub salary_range: Option<String>,
    pub experience_required: Option<String>,
    pub description: String,
    pub questions_json: String,
    pub error: String,
    pub submitted: bool,
}

impl JobApplicationPage {
    pub fn render_body(&self) -> Markup {
        let post_url = JobApplicationPublicPostRouteTag::new(self.job_form_id).url();
        html! {
            main class="container mx-auto max-w-3xl px-4 py-10" {
                h1 class="text-3xl font-bold mb-2" { (self.job_title) }
                @if let Some(salary) = &self.salary_range {
                    @if !salary.is_empty() {
                        p class="text-sm opacity-80 mb-1" { strong { "Salary range: " } (salary) }
                    }
                }
                @if let Some(exp) = &self.experience_required {
                    @if !exp.is_empty() {
                        p class="text-sm opacity-80 mb-4" { strong { "Experience: " } (exp) }
                    }
                }
                div class="prose max-w-none mb-8 whitespace-pre-wrap" { (self.description) }
                @if !self.error.is_empty() {
                    div role="alert" class="alert alert-error mb-4" { (self.error) }
                }
                form method="post" action=(post_url) class="space-y-6" {
                    input type="hidden" name="csrfmiddlewaretoken" value=(CsrfToken::current().as_str());
                    fieldset class="fieldset bg-base-200 border-base-300 rounded-box border p-4" {
                        legend class="fieldset-legend" { "Your details" }
                        label class="form-control w-full" {
                            span class="label-text" { "Name" }
                            input class="input input-bordered" type="text" name="name" required;
                        }
                        label class="form-control w-full" {
                            span class="label-text" { "Email" }
                            input class="input input-bordered" type="email" name="email" required;
                        }
                        label class="form-control w-full" {
                            span class="label-text" { "Phone" }
                            input class="input input-bordered" type="tel" name="mobile" required;
                        }
                    }
                    fieldset class="fieldset bg-base-200 border-base-300 rounded-box border p-4" {
                        legend class="fieldset-legend" { "Application questions" }
                        (input_form_answers(InputFormAnswers {
                            name: "answers_json",
                            defaults: "{}",
                            questions_json: &self.questions_json,
                            forms_catalog_json: "{}",
                            classes: "w-full",
                        }))
                    }
                    (button_submit(ButtonSubmit { label: "Submit application", ..Default::default() }))
                }
            }
        }
    }
}

pub struct JobApplicationSuccessPage {
    pub job_title: String,
}

impl JobApplicationSuccessPage {
    pub fn render_body(&self) -> Markup {
        html! {
            main class="container mx-auto max-w-2xl px-4 py-16 text-center" {
                h1 class="text-3xl font-bold mb-4" { "Application submitted" }
                p class="mb-2" { "Thank you for applying for " strong { (self.job_title) } "." }
                p class="opacity-80" {
                    "We created your portal account and emailed your login credentials. \
                     You can sign in to track your application."
                }
                div class="mt-8" {
                    a href="/users/login" class="btn btn-primary" { "Sign in" }
                }
            }
        }
    }
}
