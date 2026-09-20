use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldText,
        FormOpts, LayoutMain, LayoutSidebar, ObjectList, PaginationPage, ShellChrome,
        ShellScaffold, SidebarMenu, SidebarMenuItem, SlotCapability, SlotRegistrar, SwapKey,
        TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_clear,
        button_modal_form, button_submit, column_sort_url, container_column, container_row,
        data_table_list_refresh, delete_confirmation, detail, detail_header, field_text, form,
        form_hx_get_route, form_hx_post_selector, form_hx_post_url, label, layout_main,
        layout_sidebar, modal, modal_keyed, pagination_pages, row_attr_navigate, shell_scaffold,
        sidebar_menu, sidebar_menu_item_pane, sort_indicator, table_button_filter,
        table_pagination, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::modal_create_post_url,
};

use super::crumbs::{
    applicant_crumbs, employee_crumbs, ex_employee_crumbs, hub_crumbs, probation_crumbs,
};
use super::detail_menu::{
    applicant_detail_menu, employee_detail_menu, ex_employee_detail_menu, probation_detail_menu,
};
use super::forms::{
    ApplicantFilterForm, ApplicantFilterFormField, ApplicantForm, ApplicantFormField,
    HireEmployeeForm, StartProbationForm, TerminateEmployeeForm,
};
use super::keys::{
    ApplicantCreateModalKey, ApplicantDeleteModalKey, ApplicantEditModalKey, ApplicantHubTableKey,
    EmployeeCreateModalKey, ExEmployeeCreateModalKey, HireEmployeeModalKey,
    ProbationCreateModalKey, StartProbationModalKey, TerminateEmployeeModalKey,
};
use super::routes::{
    ApplicantCreateGetRouteTag, ApplicantCreatePostRouteTag, ApplicantDeleteGetRouteTag,
    ApplicantDeletePostRouteTag, ApplicantEditGetRouteTag, ApplicantEditPostRouteTag,
    ApplicantHubRouteTag, EmployeeCreateGetRouteTag, EmployeeCreatePostRouteTag,
    EmployeeEditGetRouteTag, EmployeeEditPostRouteTag, ExEmployeeCreateGetRouteTag,
    ExEmployeeCreatePostRouteTag, HireEmployeeGetRouteTag, HireEmployeePostRouteTag,
    ProbationCreateGetRouteTag, ProbationCreatePostRouteTag, ProbationEditGetRouteTag,
    ProbationEditPostRouteTag, StartProbationGetRouteTag, StartProbationPostRouteTag,
    TerminateEmployeeGetRouteTag, TerminateEmployeePostRouteTag,
};

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

pub fn hr_menu(active: &str) -> Markup {
    sidebar_menu(SidebarMenu {
        title: "HR",
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "People",
                url: &ApplicantHubRouteTag.url(),
                active: active == "people",
                ..Default::default()
            }))
        },
    })
}

fn tab_href(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(ApplicantHubRouteTag)
        .query("tab", tab)
        .build()
}

fn tab_nav_link(href: &str, active: bool, label: &str) -> Markup {
    use crate::components::attrs::escape_attr;
    use maud::PreEscaped;

    let cls = if active { "tab tab-active" } else { "tab" };
    let nav = crate::components::nav_content_attrs(href);
    html! {
        (PreEscaped(format!(
            r#"<a class="{cls}" href="{href}"{attrs}>"#,
            cls = escape_attr(cls),
            href = escape_attr(href),
            attrs = nav.as_string(),
        )))
        (label)
        (PreEscaped("</a>"))
    }
}

fn applicant_form_inputs(name: &str, mobile: &str, email: &str) -> Markup {
    ApplicantForm::render_inputs(
        &FormCtx::form::<ApplicantForm>(CsrfToken::current())
            .value(ApplicantFormField::Name, name)
            .value(ApplicantFormField::Mobile, mobile)
            .value(ApplicantFormField::Email, email),
    )
}

fn person_fields(name: &str, mobile: &str, email: &str) -> Markup {
    html! {
        (label("Name", field_text(FieldText { value: name, classes: "" })))
        (label("Mobile", field_text(FieldText { value: mobile, classes: "" })))
        (label("Email", field_text(FieldText { value: email, classes: "" })))
    }
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

fn render_people_data_table<K: SwapKey>(
    title: &str,
    people: &ObjectList<ApplicantRow>,
    sort: &str,
    path_and_query: &str,
    actions: Markup,
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
        TableColumnHeader {
            key: "Status",
            label: "Status",
            sort_url: None,
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
                field_text(FieldText {
                    value: &r.status,
                    classes: "",
                }),
            ],
        })
        .collect();
    let pagination = render_pagination::<K>(path_and_query, people.number, people.num_pages);
    data_table_list_refresh::<K>(title, actions, &headers, &rows, pagination, path_and_query)
}

crate::define_register_items! {
    plugin: HrTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ApplicantHubIdx: ApplicantHubPageTag => ApplicantHubPage,
        ApplicantDetailIdx: ApplicantDetailPageTag => ApplicantDetailPage,
        ApplicantCreateModalIdx: ApplicantCreateModalPageTag => ApplicantCreateModalPage,
        PersonCreateModalIdx: PersonCreateModalPageTag => PersonCreateModalPage,
        ApplicantEditModalIdx: PersonEditModalPageTag => PersonEditModalPage,
        StartProbationModalIdx: StartProbationModalPageTag => StartProbationModalPage,
        HireEmployeeModalIdx: HireEmployeeModalPageTag => HireEmployeeModalPage,
        TerminateEmployeeModalIdx: TerminateEmployeeModalPageTag => TerminateEmployeeModalPage,
        ProbationDetailIdx: ProbationDetailPageTag => ProbationDetailPage,
        EmployeeDetailIdx: EmployeeDetailPageTag => EmployeeDetailPage,
        ExEmployeeDetailIdx: ExEmployeeDetailPageTag => ExEmployeeDetailPage,
        ConfirmDeleteIdx: HrConfirmDeletePageTag => ConfirmDeletePage,
    ]
}

crate::define_register_items! {
    plugin: HrTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}

#[derive(Clone)]
pub struct ApplicantRow {
    pub id: i64,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub status: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct ApplicantHubPage {
    pub people: ObjectList<ApplicantRow>,
    pub tab: String,
    pub filter_name: String,
    pub filter_email: String,
    pub sort: String,
    pub path_and_query: String,
    pub can_edit: bool,
    pub page_size: u32,
}

impl ApplicantHubPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        tab_nav_link(&tab_href(tab), self.tab == tab, label)
    }

    pub fn render_table(&self) -> Markup {
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_get_route::<ApplicantHubTableKey, ApplicantHubRouteTag>(
                        ApplicantHubRouteTag,
                    ),
                    inputs: with_list_filter_common(
                        ApplicantFilterForm::render_inputs(
                            &FormCtx::form::<ApplicantFilterForm>(CsrfToken::current())
                                .value(ApplicantFilterFormField::Name, &self.filter_name)
                                .value(ApplicantFilterFormField::Email, &self.filter_email),
                        ),
                        self.page_size,
                    ),
                    actions: html! {
                        (container_row("flex gap-2", html! {
                            (button_submit(ButtonSubmit { label: "Apply", ..Default::default() }))
                            (button_clear(ButtonClear { label: "Clear", ..Default::default() }))
                        }))
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
        };
        if self.can_edit {
            let create_button = match self.tab.as_str() {
                "probation" => button_modal_form(ButtonModalForm {
                    name: "p_hr.ProbationCreateForm",
                    href: &ProbationCreateGetRouteTag.url(),
                    form_post_url: &ProbationCreateGetRouteTag.path(),
                    modal_uid: ProbationCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }),
                "employees" => button_modal_form(ButtonModalForm {
                    name: "p_hr.EmployeeCreateForm",
                    href: &EmployeeCreateGetRouteTag.url(),
                    form_post_url: &EmployeeCreateGetRouteTag.path(),
                    modal_uid: EmployeeCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }),
                "ex_employees" => button_modal_form(ButtonModalForm {
                    name: "p_hr.ExEmployeeCreateForm",
                    href: &ExEmployeeCreateGetRouteTag.url(),
                    form_post_url: &ExEmployeeCreateGetRouteTag.path(),
                    modal_uid: ExEmployeeCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }),
                _ => button_modal_form(ButtonModalForm {
                    name: "p_hr.ApplicantCreateForm",
                    href: &ApplicantCreateGetRouteTag.url(),
                    form_post_url: &ApplicantCreateGetRouteTag.path(),
                    modal_uid: ApplicantCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }),
            };
            actions = html! {
                (actions)
                (create_button)
            };
        }
        render_people_data_table::<ApplicantHubTableKey>(
            "People",
            &self.people,
            &self.sort,
            &self.path_and_query,
            actions,
        )
    }

    fn body(&self) -> Markup {
        html! {
            div class="tabs tabs-boxed mb-4" {
                (self.tab_link("applicants", "Applicants"))
                (self.tab_link("probation", "Probation"))
                (self.tab_link("employees", "Employees"))
                (self.tab_link("ex_employees", "Ex-employees"))
            }
            (self.render_table())
        }
    }
}

impl RenderAppPane for ApplicantHubPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(hr_menu("people"), hub_crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(hub_crumbs(), self.body())
    }
}

impl RenderTemplate for ApplicantHubPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "HR People — Lariv",
            chrome,
            hr_menu("people"),
            hub_crumbs(),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct ApplicantDetailPage {
    pub id: i64,
    pub display_name: String,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub can_edit: bool,
}

impl ApplicantDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.StartProbationForm",
                    href: &StartProbationGetRouteTag::new(self.id).url(),
                    form_post_url: &StartProbationGetRouteTag::new(self.id).path(),
                    modal_uid: StartProbationModalKey::ID,
                    label: "Start probation",
                    classes: "btn-primary",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.ApplicantEditForm",
                    href: &ApplicantEditGetRouteTag::new(self.id).url(),
                    form_post_url: &ApplicantEditPostRouteTag::new(self.id).path(),
                    modal_uid: ApplicantEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (person_fields(&self.name, &self.mobile, &self.email))
                }))
            }))
        }
    }
}

impl RenderAppPane for ApplicantDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            applicant_detail_menu(&self.display_name, self.id, "detail"),
            applicant_crumbs(&self.display_name),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(applicant_crumbs(&self.display_name), self.body())
    }
}

impl RenderTemplate for ApplicantDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Applicant — Lariv",
            chrome,
            applicant_detail_menu(&self.display_name, self.id, "detail"),
            applicant_crumbs(&self.display_name),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct ProbationDetailPage {
    pub id: i64,
    pub display_name: String,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub started_at: String,
    pub can_edit: bool,
}

impl ProbationDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.HireEmployeeForm",
                    href: &HireEmployeeGetRouteTag::new(self.id).url(),
                    form_post_url: &HireEmployeeGetRouteTag::new(self.id).path(),
                    modal_uid: HireEmployeeModalKey::ID,
                    label: "Hire as employee",
                    classes: "btn-primary",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.ApplicantEditForm",
                    href: &ProbationEditGetRouteTag::new(self.id).url(),
                    form_post_url: &ProbationEditPostRouteTag::new(self.id).path(),
                    modal_uid: ApplicantEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (label("Started at", field_text(FieldText { value: &self.started_at, classes: "" })))
                    (person_fields(&self.name, &self.mobile, &self.email))
                }))
            }))
        }
    }
}

impl RenderAppPane for ProbationDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            probation_detail_menu(&self.display_name, self.id, "detail"),
            probation_crumbs(&self.display_name),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(probation_crumbs(&self.display_name), self.body())
    }
}

impl RenderTemplate for ProbationDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Probation — Lariv",
            chrome,
            probation_detail_menu(&self.display_name, self.id, "detail"),
            probation_crumbs(&self.display_name),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct EmployeeDetailPage {
    pub id: i64,
    pub display_name: String,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub hired_at: String,
    pub can_edit: bool,
}

impl EmployeeDetailPage {
    fn body(&self) -> Markup {
        let actions = if self.can_edit {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.TerminateEmployeeForm",
                    href: &TerminateEmployeeGetRouteTag::new(self.id).url(),
                    form_post_url: &TerminateEmployeeGetRouteTag::new(self.id).path(),
                    modal_uid: TerminateEmployeeModalKey::ID,
                    label: "Terminate employment",
                    classes: "btn-primary",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.ApplicantEditForm",
                    href: &EmployeeEditGetRouteTag::new(self.id).url(),
                    form_post_url: &EmployeeEditPostRouteTag::new(self.id).path(),
                    modal_uid: ApplicantEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (label("Hired at", field_text(FieldText { value: &self.hired_at, classes: "" })))
                    (person_fields(&self.name, &self.mobile, &self.email))
                }))
            }))
        }
    }
}

impl RenderAppPane for EmployeeDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            employee_detail_menu(&self.display_name, self.id, "detail"),
            employee_crumbs(&self.display_name),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(employee_crumbs(&self.display_name), self.body())
    }
}

impl RenderTemplate for EmployeeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Employee — Lariv",
            chrome,
            employee_detail_menu(&self.display_name, self.id, "detail"),
            employee_crumbs(&self.display_name),
            self.body(),
        )
    }
}

#[derive(Generic)]
pub struct ExEmployeeDetailPage {
    pub id: i64,
    pub display_name: String,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub terminated_at: String,
}

impl ExEmployeeDetailPage {
    fn body(&self) -> Markup {
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions: html! {},
                    }))
                    (label("Terminated at", field_text(FieldText { value: &self.terminated_at, classes: "" })))
                    (person_fields(&self.name, &self.mobile, &self.email))
                }))
            }))
        }
    }
}

impl RenderAppPane for ExEmployeeDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            ex_employee_detail_menu(&self.display_name, self.id, "detail"),
            ex_employee_crumbs(&self.display_name),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(ex_employee_crumbs(&self.display_name), self.body())
    }
}

impl RenderTemplate for ExEmployeeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Ex-employee — Lariv",
            chrome,
            ex_employee_detail_menu(&self.display_name, self.id, "detail"),
            ex_employee_crumbs(&self.display_name),
            self.body(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonCreateKind {
    Probation,
    Employee,
    ExEmployee,
}

#[derive(Generic)]
pub struct PersonCreateModalPage {
    pub kind: PersonCreateKind,
    pub form_name: String,
    pub refresh_table: String,
    pub title: String,
    pub submit_label: String,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub error: String,
}

impl PersonCreateModalPage {
    pub fn new(
        form_name: String,
        refresh_table: String,
        title: &str,
        submit_label: &str,
        kind: PersonCreateKind,
    ) -> Self {
        Self {
            kind,
            form_name,
            refresh_table,
            title: title.to_string(),
            submit_label: submit_label.to_string(),
            name: String::new(),
            mobile: String::new(),
            email: String::new(),
            error: String::new(),
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        title: &str,
        submit_label: &str,
        kind: PersonCreateKind,
        form: &ApplicantForm,
        error: String,
    ) -> Self {
        Self {
            kind,
            form_name,
            refresh_table,
            title: title.to_string(),
            submit_label: submit_label.to_string(),
            name: form.name.clone(),
            mobile: form.mobile.clone(),
            email: form.email.clone(),
            error,
        }
    }

    fn post_url(&self) -> String {
        match self.kind {
            PersonCreateKind::Probation => modal_create_post_url(
                ProbationCreatePostRouteTag,
                &self.form_name,
                &self.refresh_table,
            ),
            PersonCreateKind::Employee => modal_create_post_url(
                EmployeeCreatePostRouteTag,
                &self.form_name,
                &self.refresh_table,
            ),
            PersonCreateKind::ExEmployee => modal_create_post_url(
                ExEmployeeCreatePostRouteTag,
                &self.form_name,
                &self.refresh_table,
            ),
        }
    }

    fn body(&self) -> Markup {
        let post_url = self.post_url();
        let modal_body = html! {
            h3 class="font-bold text-lg mb-4" { (self.title) }
            (form(&CsrfToken::current(), FormOpts {
                attrs: match self.kind {
                    PersonCreateKind::Probation => {
                        form_hx_post_url::<ProbationCreateModalKey>(&post_url)
                    }
                    PersonCreateKind::Employee => {
                        form_hx_post_url::<EmployeeCreateModalKey>(&post_url)
                    }
                    PersonCreateKind::ExEmployee => {
                        form_hx_post_url::<ExEmployeeCreateModalKey>(&post_url)
                    }
                },
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: applicant_form_inputs(&self.name, &self.mobile, &self.email),
                actions: html! {
                    (button_submit(ButtonSubmit { label: &self.submit_label, ..Default::default() }))
                },
                ..Default::default()
            }))
        };
        match self.kind {
            PersonCreateKind::Probation => {
                modal_keyed::<ProbationCreateModalKey>(&self.form_name, modal_body)
            }
            PersonCreateKind::Employee => {
                modal_keyed::<EmployeeCreateModalKey>(&self.form_name, modal_body)
            }
            PersonCreateKind::ExEmployee => {
                modal_keyed::<ExEmployeeCreateModalKey>(&self.form_name, modal_body)
            }
        }
    }
}

impl RenderTemplate for PersonCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        self.body()
    }
}

#[derive(Generic)]
pub struct ApplicantCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub error: String,
}

impl RenderTemplate for ApplicantCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<ApplicantCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New applicant" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<ApplicantCreateModalKey>(&modal_create_post_url(
                        ApplicantCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: applicant_form_inputs(&self.name, &self.mobile, &self.email),
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
pub struct PersonEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub show_delete: bool,
    pub error: String,
}

impl RenderTemplate for PersonEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = ApplicantDeleteGetRouteTag::new(self.id).url();
        let mut actions = html! {
            (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
        };
        if self.show_delete {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    label: "Delete",
                    icon_name: Some("trash"),
                    name: "p_hr.ApplicantDeleteForm",
                    href: &delete_url,
                    form_post_url: &delete_url,
                    modal_uid: ApplicantDeleteModalKey::ID,
                    classes: "btn-error",
                    ..Default::default()
                }))
            };
        }
        modal_keyed::<ApplicantEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit person" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<ApplicantEditModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: applicant_form_inputs(&self.name, &self.mobile, &self.email),
                    actions,
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct StartProbationModalPage {
    pub applicant_id: i64,
    pub form_name: String,
    pub refresh_table: String,
    pub error: String,
}

impl RenderTemplate for StartProbationModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<StartProbationModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Start probation" }
                p class="mb-4 text-sm opacity-80" {
                    "Move this applicant into probation?"
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<StartProbationModalKey>(&modal_create_post_url(
                        StartProbationPostRouteTag::new(self.applicant_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: StartProbationForm::render_inputs(
                        &FormCtx::form::<StartProbationForm>(CsrfToken::current()),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Start probation", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct HireEmployeeModalPage {
    pub probation_id: i64,
    pub form_name: String,
    pub refresh_table: String,
    pub error: String,
}

impl RenderTemplate for HireEmployeeModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<HireEmployeeModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Hire employee" }
                p class="mb-4 text-sm opacity-80" {
                    "Confirm hiring this person as a full employee?"
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<HireEmployeeModalKey>(&modal_create_post_url(
                        HireEmployeePostRouteTag::new(self.probation_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: HireEmployeeForm::render_inputs(
                        &FormCtx::form::<HireEmployeeForm>(CsrfToken::current()),
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

#[derive(Generic)]
pub struct TerminateEmployeeModalPage {
    pub employee_id: i64,
    pub form_name: String,
    pub refresh_table: String,
    pub error: String,
}

impl RenderTemplate for TerminateEmployeeModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<TerminateEmployeeModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Terminate employment" }
                p class="mb-4 text-sm opacity-80" {
                    "Move this employee to ex-employee status?"
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<TerminateEmployeeModalKey>(&modal_create_post_url(
                        TerminateEmployeePostRouteTag::new(self.employee_id),
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TerminateEmployeeForm::render_inputs(
                        &FormCtx::form::<TerminateEmployeeForm>(CsrfToken::current()),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Terminate", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
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
        let target = format!("#{}", self.modal_uid);
        let post_url = ApplicantDeletePostRouteTag::new(self.id).url();
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
