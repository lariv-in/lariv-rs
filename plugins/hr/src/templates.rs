use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::table::table_grid_content;
use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DataTable, DataTableDisplay, DeleteConfirmation, DetailHeader,
    FieldText, FilterClauseView, FormOpts, LayoutMain, LayoutSidebar, ListFilterBuilder,
    ObjectList, PaginationPage, ShellChrome, ShellScaffold, ShellTopbar, SlotCapability,
    SlotRegistrar, SwapKey, TableButtonFilter, TableColumnHeader, TableListContent,
    TablePagination, TableRow, app_layout_pane, button_modal_form, button_modal_route,
    button_submit, column_sort_url, container_column, data_table, delete_confirmation, detail,
    detail_header, field_text, filter_error_banner, form, form_hx_get_route,
    form_hx_post_multipart_url, form_hx_post_selector, form_hx_post_url, form_post_multipart,
    label, layout_main, layout_sidebar, list_filter_builder, modal, modal_keyed, pagination_pages,
    row_attr_navigate, shell_scaffold, shell_topbar, sort_indicator, table_button_columns,
    table_button_filter, table_list_content, table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, FormFieldKey, HtmlForm, render_field_specs};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{
    RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar,
};
use lariv_core::web::modal_create_post_url;

use super::crumbs::{employee_crumbs, ex_employee_crumbs, hub_crumbs};
use super::detail_menu::{employee_detail_menu, ex_employee_detail_menu};
use super::forms::{
    ApplicantForm, ApplicantFormField, EmployeeForm, EmployeeFormField, EmployeeFormFlag,
    PersonForm, PersonFormField, TerminateEmployeeForm,
};
use super::keys::{
    APPLICANT_EDIT_MODAL, APPLICANT_HUB_TABLE, EMPLOYEE_CREATE_MODAL, EMPLOYEE_DELETE_MODAL,
    EX_EMPLOYEE_CREATE_MODAL, TERMINATE_EMPLOYEE_MODAL,
};
use super::logic::dashboard::MissingHrProfile;
use super::logic::profile::EmployeeProfileView;
use super::routes::{
    EmployeeCreateGetRouteTag, EmployeeCreatePostRouteTag, EmployeeDeleteGetRouteTag,
    EmployeeEditGetRouteTag, EmployeeEditPostRouteTag, EmployeeIdCardModalRouteTag,
    ExEmployeeCreateGetRouteTag, ExEmployeeCreatePostRouteTag, HrDashboardPostRouteTag,
    PeopleHubRouteTag, TerminateEmployeeGetRouteTag, TerminateEmployeePostRouteTag,
};

pub fn app_scaffold(
    title: &str,
    chrome: &ShellChrome,
    sidebar: Option<Markup>,
    crumbs: Markup,
    body: Markup,
) -> Markup {
    match sidebar {
        Some(sidebar) => shell_scaffold(ShellScaffold {
            title,
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar,
            breadcrumbs: crumbs,
            body,
            ..Default::default()
        }),
        // Disallowed roles: topbar-only shell (no left nav, no right chrome drawer).
        None => shell_topbar(ShellTopbar {
            title,
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: Markup::default(),
            body: html! {
                div class="p-4" {
                    (crumbs)
                    (body)
                }
            },
            ..Default::default()
        }),
    }
}

pub fn scaffold_pane(
    sidebar: Option<Markup>,
    crumbs: Markup,
    body: Markup,
) -> lariv_core::components::AppLayoutHtml {
    match sidebar {
        Some(sidebar) => layout_sidebar(LayoutSidebar {
            sidebar,
            breadcrumbs: crumbs,
            content: body,
        }),
        None => app_layout_pane(html! {
            (crumbs)
            (body)
        }),
    }
}

pub fn scaffold_main(crumbs: Markup, body: Markup) -> lariv_core::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

/// HR sidebar, rendered from the shared registry.
pub fn hr_menu(active: &str) -> Option<Markup> {
    super::hr_sidebar::hr_menu_for_active(active)
}

pub fn detail_sidebar(menu: Markup, roles: &[String]) -> Option<Markup> {
    let gated = lariv_core::components::authorized_role(roles, menu);
    let raw = gated.into_string();
    if raw.trim().is_empty() {
        None
    } else {
        Some(maud::PreEscaped(raw))
    }
}

pub mod preferences;

fn tab_href(tab: &str) -> String {
    lariv_core::http::RouteQueryBuilder::new(PeopleHubRouteTag)
        .query("tab", tab)
        .build()
}

pub fn tab_nav_link(href: &str, active: bool, label: &str) -> Markup {
    use lariv_core::components::attrs::escape_attr;
    use maud::PreEscaped;

    let cls = if active { "tab tab-active" } else { "tab" };
    let nav = lariv_core::components::nav_content_attrs(href);
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

#[derive(Clone, Default)]
pub struct EmployeeFormValues {
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub fathers_name: String,
    pub date_of_birth: String,
    pub gender: String,
    pub marital_status: String,
    pub nationality: String,
    pub is_disabled: bool,
    pub disability_type: String,
    pub photograph_display: String,
    pub blood_group: String,
    pub identification_mark: String,
    pub present_address: String,
    pub present_pin_code: String,
    pub same_as_present: bool,
    pub permanent_address: String,
    pub permanent_pin_code: String,
    pub emergency_contact_name: String,
    pub emergency_contact_relation: String,
    pub emergency_contact_mobile: String,
    pub pf_number: String,
    pub aadhar_number: String,
    pub pan_number: String,
    pub aadhar_display: String,
    pub pan_display: String,
    pub passport_display: String,
    pub account_holder_name: String,
    pub account_number: String,
    pub account_ifsc_code: String,
    pub account_type: String,
    pub qualifications: String,
    pub verified: bool,
    pub date_of_joining: String,
    pub probation_end_date: String,
    pub work_start: String,
    pub work_end: String,
    pub base_salary: String,
    pub hourly_wage: String,
    pub manager_id: String,
    pub manager_display: String,
    pub user_id: String,
    pub user_display: String,
    pub employee_code: String,
    /// Admin is editing someone else's employee record.
    pub can_edit_manager: bool,
    /// The employee user is a superuser, so manager cannot be empty.
    pub manager_required: bool,
}

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

fn employee_form_inputs(values: &EmployeeFormValues, include_admin_dates: bool) -> Markup {
    let gender = choice_pairs(EmployeeForm::gender_choices());
    let marital = choice_pairs(EmployeeForm::marital_status_choices());
    let nationality = choice_pairs(EmployeeForm::nationality_choices());
    let disability = choice_pairs(EmployeeForm::disability_type_choices());
    let blood = choice_pairs(EmployeeForm::blood_group_choices());
    let account = choice_pairs(EmployeeForm::account_type_choices());

    let photo_hint = if values.photograph_display.is_empty() {
        String::new()
    } else {
        format!("Current file: {}", values.photograph_display)
    };
    let aadhar_hint = if values.aadhar_display.is_empty() {
        String::new()
    } else {
        format!("Current file: {}", values.aadhar_display)
    };
    let pan_hint = if values.pan_display.is_empty() {
        String::new()
    } else {
        format!("Current file: {}", values.pan_display)
    };
    let passport_hint = if values.passport_display.is_empty() {
        String::new()
    } else {
        format!("Current file: {}", values.passport_display)
    };

    let mut ctx = FormCtx::form::<EmployeeForm>(CsrfToken::current())
        .value(EmployeeFormField::Name, &values.name)
        .value(EmployeeFormField::Mobile, &values.mobile)
        .value(EmployeeFormField::Email, &values.email)
        .value(EmployeeFormField::FathersName, &values.fathers_name)
        .value(EmployeeFormField::DateOfBirth, &values.date_of_birth)
        .value(EmployeeFormField::Gender, &values.gender)
        .value(EmployeeFormField::MaritalStatus, &values.marital_status)
        .value(EmployeeFormField::Nationality, &values.nationality)
        .checked(EmployeeFormField::IsDisabled, values.is_disabled)
        .value(EmployeeFormField::DisabilityType, &values.disability_type)
        .value(EmployeeFormField::BloodGroup, &values.blood_group)
        .value(
            EmployeeFormField::IdentificationMark,
            &values.identification_mark,
        )
        .value(EmployeeFormField::PresentAddress, &values.present_address)
        .value(EmployeeFormField::PresentPinCode, &values.present_pin_code)
        .checked(EmployeeFormField::SameAsPresent, values.same_as_present)
        .value(
            EmployeeFormField::PermanentAddress,
            &values.permanent_address,
        )
        .value(
            EmployeeFormField::PermanentPinCode,
            &values.permanent_pin_code,
        )
        .value(
            EmployeeFormField::EmergencyContactName,
            &values.emergency_contact_name,
        )
        .value(
            EmployeeFormField::EmergencyContactRelation,
            &values.emergency_contact_relation,
        )
        .value(
            EmployeeFormField::EmergencyContactMobile,
            &values.emergency_contact_mobile,
        )
        .value(EmployeeFormField::PfNumber, &values.pf_number)
        .value(EmployeeFormField::AadharNumber, &values.aadhar_number)
        .value(EmployeeFormField::PanNumber, &values.pan_number)
        .value(
            EmployeeFormField::AccountHolderName,
            &values.account_holder_name,
        )
        .value(EmployeeFormField::AccountNumber, &values.account_number)
        .value(
            EmployeeFormField::AccountIfscCode,
            &values.account_ifsc_code,
        )
        .value(EmployeeFormField::AccountType, &values.account_type)
        .value(EmployeeFormField::Qualifications, &values.qualifications)
        .checked(EmployeeFormField::Verified, values.verified)
        .value(EmployeeFormField::DateOfJoining, &values.date_of_joining)
        .value(
            EmployeeFormField::ProbationEndDate,
            &values.probation_end_date,
        )
        .value(EmployeeFormField::WorkStart, &values.work_start)
        .value(EmployeeFormField::WorkEnd, &values.work_end)
        .value(EmployeeFormField::BaseSalary, &values.base_salary)
        .value(EmployeeFormField::HourlyWage, &values.hourly_wage)
        .value(EmployeeFormField::ManagerId, &values.manager_id)
        .display(EmployeeFormField::ManagerId, &values.manager_display)
        .value(EmployeeFormField::UserId, &values.user_id)
        .display(EmployeeFormField::UserId, &values.user_display)
        .value(EmployeeFormField::EmployeeCode, &values.employee_code)
        .choices(EmployeeFormField::Gender, &gender)
        .choices(EmployeeFormField::MaritalStatus, &marital)
        .choices(EmployeeFormField::Nationality, &nationality)
        .choices(EmployeeFormField::DisabilityType, &disability)
        .choices(EmployeeFormField::BloodGroup, &blood)
        .choices(EmployeeFormField::AccountType, &account);
    if include_admin_dates {
        ctx = ctx
            .flag(EmployeeFormFlag::AdminDates, true)
            .flag(EmployeeFormFlag::PickUser, true);
    } else {
        ctx = ctx
            .flag(EmployeeFormFlag::CollectIdentity, true)
            .label(EmployeeFormField::IsDisabled, "Do you have any disability");
    }
    if values.can_edit_manager {
        ctx = ctx.flag(EmployeeFormFlag::EditManager, true);
    }
    if !values.manager_required {
        ctx = ctx.flag(EmployeeFormFlag::ManagerOptional, true);
    }

    if !photo_hint.is_empty() {
        ctx = ctx.hint(EmployeeFormField::Photograph, &photo_hint);
    }
    if !aadhar_hint.is_empty() {
        ctx = ctx.hint(EmployeeFormField::Aadhar, &aadhar_hint);
    }
    if !pan_hint.is_empty() {
        ctx = ctx.hint(EmployeeFormField::Pan, &pan_hint);
    }
    if !passport_hint.is_empty() {
        ctx = ctx.hint(EmployeeFormField::Passport, &passport_hint);
    }

    let rendered = if include_admin_dates {
        EmployeeForm::render_inputs(&ctx)
    } else {
        render_employee_self_service_inputs(&ctx)
    };

    let is_disabled = serde_json::to_string(&values.is_disabled).unwrap_or_else(|_| "false".into());
    let same_as_present =
        serde_json::to_string(&values.same_as_present).unwrap_or_else(|_| "false".into());
    let present_address =
        serde_json::to_string(&values.present_address).unwrap_or_else(|_| "\"\"".into());
    let present_pin_code =
        serde_json::to_string(&values.present_pin_code).unwrap_or_else(|_| "\"\"".into());
    let permanent_address =
        serde_json::to_string(&values.permanent_address).unwrap_or_else(|_| "\"\"".into());
    let permanent_pin_code =
        serde_json::to_string(&values.permanent_pin_code).unwrap_or_else(|_| "\"\"".into());

    let alpine_init = format!(
        r#"{{
    is_disabled: {is_disabled},
    same_as_present: {same_as_present},
    present_address: {present_address},
    present_pin_code: {present_pin_code},
    permanent_address: {permanent_address},
    permanent_pin_code: {permanent_pin_code},
    init() {{
        this.$watch('same_as_present', val => {{
            if (val) {{
                this.permanent_address = this.present_address;
                this.permanent_pin_code = this.present_pin_code;
            }}
        }});
        this.$watch('present_address', val => {{
            if (this.same_as_present) {{
                this.permanent_address = val;
            }}
        }});
        this.$watch('present_pin_code', val => {{
            if (this.same_as_present) {{
                this.permanent_pin_code = val;
            }}
        }});
    }}
}}"#
    );

    html! {
        div x-data=(alpine_init) {
            (rendered)
        }
    }
}

/// Employee self-service keeps the disability question, and the type that follows it, last.
fn render_employee_self_service_inputs(ctx: &FormCtx<'_>) -> Markup {
    let disability = EmployeeFormField::IsDisabled.html_name();
    let disability_type = EmployeeFormField::DisabilityType.html_name();
    let mut head = Vec::new();
    let mut tail = Vec::new();
    for spec in EmployeeForm::field_specs() {
        if spec.name == disability || spec.name == disability_type {
            tail.push(*spec);
        } else {
            head.push(*spec);
        }
    }
    head.extend(tail);
    render_field_specs(&head, ctx)
}

fn linked_value(href: &str, label: &str) -> Markup {
    if href.is_empty() {
        field_text(FieldText {
            value: label,
            classes: "",
        })
    } else {
        html! { a class="link link-primary" href=(href) { (label) } }
    }
}

fn employee_profile_fields(profile: &EmployeeProfileView) -> Markup {
    html! {
        (label("Father's name", field_text(FieldText { value: &profile.fathers_name, classes: "" })))
        (label("Date of birth", field_text(FieldText { value: &profile.date_of_birth, classes: "" })))
        (label("Gender", field_text(FieldText { value: &profile.gender, classes: "" })))
        (label("Marital status", field_text(FieldText { value: &profile.marital_status, classes: "" })))
        (label("Nationality", field_text(FieldText { value: &profile.nationality, classes: "" })))
        (label("Is disabled", field_text(FieldText { value: &profile.is_disabled, classes: "" })))
        @if !profile.disability_type.is_empty() {
            (label("Disability type", field_text(FieldText { value: &profile.disability_type, classes: "" })))
        }
        (label("Photograph", linked_value(&profile.photograph_href, &profile.photograph_name)))
        (label("Blood group", field_text(FieldText { value: &profile.blood_group, classes: "" })))
        (label("Identification mark", field_text(FieldText { value: &profile.identification_mark, classes: "" })))
        (label("Present address", field_text(FieldText { value: &profile.present_address, classes: "" })))
        (label("Present PIN code", field_text(FieldText { value: &profile.present_pin_code, classes: "" })))
        (label("Permanent address", field_text(FieldText { value: &profile.permanent_address, classes: "" })))
        (label("Permanent PIN code", field_text(FieldText { value: &profile.permanent_pin_code, classes: "" })))
        (label("Emergency contact name", field_text(FieldText { value: &profile.emergency_contact_name, classes: "" })))
        (label("Emergency contact relation", field_text(FieldText { value: &profile.emergency_contact_relation, classes: "" })))
        (label("Emergency contact mobile", field_text(FieldText { value: &profile.emergency_contact_mobile, classes: "" })))
        (label("PF Number", field_text(FieldText { value: &profile.pf_number, classes: "" })))
        (label("Aadhaar Number", field_text(FieldText { value: &profile.aadhar_number, classes: "" })))
        (label("PAN Number", field_text(FieldText { value: &profile.pan_number, classes: "" })))
        (label("Aadhar", linked_value(&profile.aadhar_href, &profile.aadhar_label)))
        (label("PAN", linked_value(&profile.pan_href, &profile.pan_label)))
        (label("Passport", linked_value(&profile.passport_href, &profile.passport_label)))
        (label("Bank account holder name", field_text(FieldText { value: &profile.account_holder_name, classes: "" })))
        (label("Bank account number", field_text(FieldText { value: &profile.account_number, classes: "" })))
        (label("Bank account IFSC code", field_text(FieldText { value: &profile.account_ifsc_code, classes: "" })))
        (label("Bank account type", field_text(FieldText { value: &profile.account_type, classes: "" })))
        (label("Qualifications", field_text(FieldText { value: &profile.qualifications, classes: "" })))
        (label("Verified", field_text(FieldText { value: &profile.verified, classes: "" })))
        (label("Manager", field_text(FieldText { value: &profile.manager, classes: "" })))
        (label("Date of joining", field_text(FieldText { value: &profile.date_of_joining, classes: "" })))
        (label("Probation end date", field_text(FieldText { value: &profile.probation_end_date, classes: "" })))
        (label("Work start", field_text(FieldText { value: &profile.work_start, classes: "" })))
        (label("Work end", field_text(FieldText { value: &profile.work_end, classes: "" })))
        (label("Base salary", field_text(FieldText { value: &profile.base_salary, classes: "" })))
        (label("Hourly wage", field_text(FieldText { value: &profile.hourly_wage, classes: "" })))
    }
}

fn person_form_inputs(name: &str, mobile: &str, email: &str) -> Markup {
    PersonForm::render_inputs(
        &FormCtx::form::<PersonForm>(CsrfToken::current())
            .value(PersonFormField::Name, name)
            .value(PersonFormField::Mobile, mobile)
            .value(PersonFormField::Email, email),
    )
}

fn gender_choice_pairs() -> Vec<(String, String)> {
    ApplicantForm::gender_choices()
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
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
    let choices = gender_choice_pairs();
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

fn render_pagination(key: SwapKey, path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let hx_target = key.selector();
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
        hx_target: &hx_target,
    })
}

pub fn data_table_with_filter_errors(
    key: SwapKey,
    title: &str,
    actions: Markup,
    headers: &[TableColumnHeader<'_>],
    rows: &[TableRow],
    pagination: Markup,
    refresh_url: &str,
    errors: &[FilterRowError],
) -> Markup {
    let hx_target = key.selector();
    let list = html! {
        (filter_error_banner(errors))
        (table_list_content(TableListContent {
            headers,
            rows,
            hx_target: &hx_target,
        }))
    };
    let grid = html! {
        (filter_error_banner(errors))
        (table_grid_content(headers, rows))
    };
    let column_keys: Vec<&str> = headers
        .iter()
        .map(|header| header.key)
        .filter(|key| !key.is_empty())
        .collect();
    let toolbar = if headers.len() > 1 {
        html! {
            (table_button_columns(headers))
            (actions)
        }
    } else {
        actions
    };
    data_table(DataTable {
        uid: key.id(),
        title,
        classes: "w-full",
        actions: toolbar,
        displays: vec![
            DataTableDisplay {
                name: "Grid".into(),
                html: grid,
            },
            DataTableDisplay {
                name: "List".into(),
                html: list,
            },
        ],
        pagination,
        refresh_url,
        column_keys: &column_keys,
        ..Default::default()
    })
}

fn render_people_data_table(
    key: SwapKey,
    title: &str,
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
    let show_code = people
        .items
        .iter()
        .any(|row| row.status == "Employee" || !row.employee_code.is_empty());
    let mut headers = Vec::new();
    if show_code {
        headers.push(TableColumnHeader {
            key: "Employee code",
            label: "Employee code",
            sort_url: None,
            push_url: true,
        });
    }
    headers.extend([
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
    ]);
    let rows: Vec<TableRow> = people
        .items
        .iter()
        .map(|r| {
            let mut cells = Vec::new();
            if show_code {
                cells.push(field_text(FieldText {
                    value: &r.employee_code,
                    classes: "",
                }));
            }
            cells.extend([
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
            ]);
            TableRow {
                attrs: row_attr_navigate(&r.detail_href),
                cells,
                key: String::new(),
                children: Vec::new(),
            }
        })
        .collect();
    let pagination = render_pagination(key, path_and_query, people.number, people.num_pages);
    data_table_with_filter_errors(
        key,
        title,
        actions,
        &headers,
        &rows,
        pagination,
        path_and_query,
        errors,
    )
}

lariv_core::define_register_items! {
    plugin: HrTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ApplicantHubIdx: ApplicantHubPageTag => ApplicantHubPage,        PersonCreateModalIdx: PersonCreateModalPageTag => PersonCreateModalPage,
        PersonEditModalIdx: PersonEditModalPageTag => PersonEditModalPage,
        TerminateEmployeeModalIdx: TerminateEmployeeModalPageTag => TerminateEmployeeModalPage,
        EmployeeDetailIdx: EmployeeDetailPageTag => EmployeeDetailPage,        ExEmployeeDetailIdx: ExEmployeeDetailPageTag => ExEmployeeDetailPage,
        ConfirmDeleteIdx: HrConfirmDeletePageTag => ConfirmDeletePage,        HrDashboardGateIdx: HrDashboardGatePageTag => HrDashboardGatePage,
        HrDashboardSuccessIdx: HrDashboardSuccessPageTag => HrDashboardSuccessPage,
    ]
}

lariv_core::define_register_items! {
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
    pub employee_code: String,
    pub status: String,
    pub detail_href: String,
}

#[derive(Generic)]
pub struct ApplicantHubPage {
    pub people: ObjectList<ApplicantRow>,
    pub tab: String,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
}

impl ApplicantHubPage {
    fn tab_link(&self, tab: &str, label: &str) -> Markup {
        tab_nav_link(&tab_href(tab), self.tab == tab, label)
    }

    pub fn render_table(&self) -> Markup {
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route(
                        PeopleHubRouteTag,
                    ),
                    apply_label: "Apply",
                    extra: html! {
                        input type="hidden" name="tab" value=(self.tab) {}
                    },
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<super::routes::EmployeeMutate>(),
        ) {
            let create_button = match self.tab.as_str() {
                "ex_employees" => button_modal_form(ButtonModalForm {
                    name: "p_hr.ExEmployeeCreateForm",
                    href: &ExEmployeeCreateGetRouteTag.url(),
                    form_post_url: &ExEmployeeCreateGetRouteTag.path(),
                    modal_uid: EX_EMPLOYEE_CREATE_MODAL.id(),
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }),
                _ => button_modal_form(ButtonModalForm {
                    name: "p_hr.EmployeeCreateForm",
                    href: &EmployeeCreateGetRouteTag.url(),
                    form_post_url: &EmployeeCreateGetRouteTag.path(),
                    modal_uid: EMPLOYEE_CREATE_MODAL.id(),
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
        render_people_data_table(
            APPLICANT_HUB_TABLE,
            "People",
            &self.people,
            &self.sort,
            &self.path_and_query,
            actions,
            &self.filter_errors,
        )
    }

    fn body(&self) -> Markup {
        html! {
            div class="tabs tabs-boxed mb-4" {
                (self.tab_link("employees", "Employees"))
                (self.tab_link("ex_employees", "Ex-employees"))
            }
            (self.render_table())
        }
    }
}

impl RenderAppPane for ApplicantHubPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(hr_menu("people"), hub_crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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
pub struct EmployeeDetailPage {
    pub id: i64,
    pub display_name: String,
    pub employee_code: String,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub profile: EmployeeProfileView,
}

impl EmployeeDetailPage {
    fn body(&self) -> Markup {
        let can_mutate = lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<super::routes::EmployeeMutate>(),
        );
        let actions = html! {
            (button_modal_route(
                EmployeeIdCardModalRouteTag::new(self.id),
                "ID Card",
                "btn-outline",
            ))
            @if can_mutate {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.TerminateEmployeeForm",
                    href: &TerminateEmployeeGetRouteTag::new(self.id).url(),
                    form_post_url: &TerminateEmployeeGetRouteTag::new(self.id).path(),
                    modal_uid: TERMINATE_EMPLOYEE_MODAL.id(),
                    label: "Terminate employment",
                    classes: "btn-primary",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.ApplicantEditForm",
                    href: &EmployeeEditGetRouteTag::new(self.id).url(),
                    form_post_url: &EmployeeEditPostRouteTag::new(self.id).path(),
                    modal_uid: APPLICANT_EDIT_MODAL.id(),
                    label: "Edit",
                    classes: "btn-outline",
                    ..Default::default()
                }))
            }
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (label("Employee code", field_text(FieldText { value: &self.employee_code, classes: "" })))
                    (person_fields(&self.name, &self.mobile, &self.email))
                    (employee_profile_fields(&self.profile))
                }))
            }))
        }
    }

    fn menu(&self) -> Option<Markup> {
        detail_sidebar(
            employee_detail_menu(&self.display_name, self.id, "detail"),
            &lariv_plugin_users::role_authorization::roles_for::<super::routes::EmployeeMutate>(),
        )
    }

    fn crumbs(&self) -> Markup {
        employee_crumbs(&self.display_name)
    }
}

impl RenderAppPane for EmployeeDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(self.menu(), self.crumbs(), self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.body())
    }
}

impl RenderTemplate for EmployeeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Employee — Lariv",
            chrome,
            self.menu(),
            self.crumbs(),
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
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            detail_sidebar(
                ex_employee_detail_menu(&self.display_name, self.id, "detail"),
                &lariv_plugin_users::role_authorization::roles_for::<super::routes::ExEmployeeMutate>(
                ),
            ),
            ex_employee_crumbs(&self.display_name),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(ex_employee_crumbs(&self.display_name), self.body())
    }
}

impl RenderTemplate for ExEmployeeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Ex-employee — Lariv",
            chrome,
            detail_sidebar(
                ex_employee_detail_menu(&self.display_name, self.id, "detail"),
                &lariv_plugin_users::role_authorization::roles_for::<super::routes::ExEmployeeMutate>(
                ),
            ),
            ex_employee_crumbs(&self.display_name),
            self.body(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonCreateKind {
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
    pub employee: EmployeeFormValues,
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
            employee: EmployeeFormValues {
                can_edit_manager: kind == PersonCreateKind::Employee,
                ..EmployeeFormValues::default()
            },
            error: String::new(),
        }
    }

    pub fn with_employee(
        form_name: String,
        refresh_table: String,
        title: &str,
        submit_label: &str,
        kind: PersonCreateKind,
        employee: EmployeeFormValues,
        error: String,
    ) -> Self {
        let mut employee = employee;
        if kind == PersonCreateKind::Employee {
            employee.can_edit_manager = true;
        }
        Self {
            kind,
            form_name,
            refresh_table,
            title: title.to_string(),
            submit_label: submit_label.to_string(),
            name: employee.name.clone(),
            mobile: employee.mobile.clone(),
            email: employee.email.clone(),
            employee,
            error,
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        title: &str,
        submit_label: &str,
        kind: PersonCreateKind,
        form: &PersonForm,
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
            employee: EmployeeFormValues::default(),
            error,
        }
    }

    fn post_url(&self) -> String {
        match self.kind {
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
                    PersonCreateKind::Employee => {
                        form_hx_post_multipart_url(EMPLOYEE_CREATE_MODAL, &post_url)
                    }
                    PersonCreateKind::ExEmployee => {
                        form_hx_post_url(EX_EMPLOYEE_CREATE_MODAL, &post_url)
                    }
                },
                enctype: if self.kind == PersonCreateKind::Employee {
                    Some("multipart/form-data")
                } else {
                    None
                },
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: if self.kind == PersonCreateKind::ExEmployee {
                    person_form_inputs(&self.name, &self.mobile, &self.email)
                } else {
                    employee_form_inputs(&self.employee, true)
                },
                actions: html! {
                    (button_submit(ButtonSubmit { label: &self.submit_label, ..Default::default() }))
                },
                ..Default::default()
            }))
        };
        match self.kind {
            PersonCreateKind::Employee => {
                modal_keyed(EMPLOYEE_CREATE_MODAL, &self.form_name, modal_body)
            }
            PersonCreateKind::ExEmployee => {
                modal_keyed(EX_EMPLOYEE_CREATE_MODAL, &self.form_name, modal_body)
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

pub struct PersonEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub values: EmployeeFormValues,
    pub show_delete: bool,
    pub error: String,
}

impl RenderTemplate for PersonEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = EmployeeDeleteGetRouteTag::new(self.id).url();
        let mut actions = html! {
            (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
        };
        if self.show_delete {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    label: "Delete",
                    icon_name: Some("trash"),
                    name: "p_hr.EmployeeDeleteForm",
                    href: &delete_url,
                    form_post_url: &delete_url,
                    modal_uid: EMPLOYEE_DELETE_MODAL.id(),
                    classes: "btn-error",
                    ..Default::default()
                }))
            };
        }
        modal_keyed(
            APPLICANT_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit employee" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_multipart_url(APPLICANT_EDIT_MODAL, &self.post_url),
                    enctype: Some("multipart/form-data"),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: employee_form_inputs(&self.values, true),
                    actions,
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
        modal_keyed(
            TERMINATE_EMPLOYEE_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Terminate employment" }
                p class="mb-4 text-sm opacity-80" {
                    "Move this employee to ex-employee status?"
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(TERMINATE_EMPLOYEE_MODAL, &modal_create_post_url(
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
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", self.modal_uid);
        modal(lariv_core::components::Modal {
            uid: self.modal_uid.as_str(),
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&self.post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[derive(Clone, Generic)]
pub struct HrDashboardGatePage {
    pub kind: MissingHrProfile,
    pub applicant: ApplicantFormValues,
    pub employee: EmployeeFormValues,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub error: String,
}

impl HrDashboardGatePage {
    pub fn for_applicant(values: ApplicantFormValues, error: String) -> Self {
        Self {
            kind: MissingHrProfile::Applicant,
            applicant: values,
            employee: EmployeeFormValues::default(),
            name: String::new(),
            mobile: String::new(),
            email: String::new(),
            error,
        }
    }

    pub fn for_employee(kind: MissingHrProfile, values: EmployeeFormValues, error: String) -> Self {
        Self {
            kind,
            applicant: ApplicantFormValues::default(),
            employee: values,
            name: String::new(),
            mobile: String::new(),
            email: String::new(),
            error,
        }
    }

    pub fn for_ex_employee(name: String, mobile: String, email: String, error: String) -> Self {
        Self {
            kind: MissingHrProfile::ExEmployee,
            applicant: ApplicantFormValues::default(),
            employee: EmployeeFormValues::default(),
            name,
            mobile,
            email,
            error,
        }
    }

    fn title(&self) -> &'static str {
        match self.kind {
            MissingHrProfile::Applicant => "Applicant details",
            MissingHrProfile::Employee => "Employee details",
            MissingHrProfile::ExEmployee => "Ex-employee details",
        }
    }

    fn submit_label(&self) -> &'static str {
        match self.kind {
            MissingHrProfile::Applicant => "Submit applicant details",
            MissingHrProfile::Employee => "Submit employee details",
            MissingHrProfile::ExEmployee => "Submit ex-employee details",
        }
    }

    fn body(&self) -> Markup {
        let inputs = match self.kind {
            MissingHrProfile::Applicant => applicant_form_inputs(&self.applicant),
            MissingHrProfile::Employee => employee_form_inputs(&self.employee, false),
            MissingHrProfile::ExEmployee => {
                person_form_inputs(&self.name, &self.mobile, &self.email)
            }
        };

        html! {
            div class="container max-w-2xl mx-auto py-6" {
                div class="card bg-base-100 shadow-xl border border-base-200" {
                    div class="card-body p-6 sm:p-8" {
                        h1 class="card-title text-2xl font-bold mb-1" { (self.title()) }
                        p class="text-sm text-base-content/70 mb-6" {
                            "Please complete your profile details to access the dashboard."
                        }
                        // Classic multipart POST — HTMX urlencoded conversion drops file parts.
                        (form(&CsrfToken::current(), FormOpts {
                            attrs: form_post_multipart(&HrDashboardPostRouteTag.path()),
                            enctype: Some("multipart/form-data"),
                            form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                            inputs,
                            actions: html! {
                                div class="flex justify-end gap-3 mt-6 pt-4 border-t border-base-200" {
                                    (button_submit(ButtonSubmit {
                                        label: self.submit_label(),
                                        classes: "btn-primary",
                                        ..Default::default()
                                    }))
                                }
                            },
                            ..Default::default()
                        }))
                    }
                }
            }
        }
    }
}

impl RenderAppPane for HrDashboardGatePage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        app_layout_pane(self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: Markup::default(),
            content: self.body(),
        })
    }
}

impl RenderTemplate for HrDashboardGatePage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_topbar(ShellTopbar {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            // HR self-service roles: no chrome right drawer.
            right_sidebar: Markup::default(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[derive(Clone, Generic)]
pub struct HrDashboardSuccessPage {
    pub message: String,
}

impl Default for HrDashboardSuccessPage {
    fn default() -> Self {
        Self {
            message: "Your form has been submitted successfully!".to_string(),
        }
    }
}

impl HrDashboardSuccessPage {
    pub fn new() -> Self {
        Self::default()
    }

    fn body(&self) -> Markup {
        let msg = if self.message.is_empty() {
            "Your form has been submitted successfully!"
        } else {
            &self.message
        };
        html! {
            div class="container max-w-xl mx-auto py-12" {
                div class="card bg-base-100 shadow-xl border border-base-200 text-center p-8" {
                    div class="flex justify-center mb-4" {
                        div class="rounded-full bg-success/20 p-4 text-success" {
                            (lariv_core::components::text::icon("check-circle", "w-12 h-12"))
                        }
                    }
                    h1 class="text-2xl font-bold mb-2" {
                        (msg)
                    }
                    p class="text-base-content/70" {
                        "Your information has been recorded in the HR system."
                    }
                }
            }
        }
    }
}

impl RenderAppPane for HrDashboardSuccessPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        app_layout_pane(self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: Markup::default(),
            content: self.body(),
        })
    }
}

impl RenderTemplate for HrDashboardSuccessPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_topbar(ShellTopbar {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            // HR self-service roles: no chrome right drawer.
            right_sidebar: Markup::default(),
            body: self.body(),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::hr_menu;
    use crate::routes::{HrPeopleView, JobFormView, LeaveView};
    use lariv_plugin_users::role_authorization::{RoleAuthorizationRegistry, with_principal};
    use lariv_plugin_users::roles::{Superuser, Unassigned};
    use lariv_plugin_users::state::{AuthContext, AuthUser};

    fn auth(role: &str) -> AuthContext {
        AuthContext {
            user: AuthUser {
                id: 1,
                name: "Test".into(),
                email: "t@example.com".into(),
                phone: "+10000000000".into(),
                password_hash: Some(vec![]),
                password_salt: Some(vec![]),
                timezone: "UTC".into(),
                last_login_time: None,
            },
            role: role.into(),
            timezone: "UTC".into(),
        }
    }

    #[test]
    fn granted_role_sees_leaves_in_the_hr_sidebar() {
        lariv_plugin_users::role_authorization::register_core_auth_hooks();
        let registry = RoleAuthorizationRegistry::new()
            .allow::<HrPeopleView>(vec![Unassigned::NAME.into()])
            .allow::<JobFormView>(vec![Unassigned::NAME.into()])
            .allow::<LeaveView>(vec![Unassigned::NAME.into()]);
        let html = with_principal(auth(Unassigned::NAME), registry.clone(), || {
            hr_menu("people").expect("sidebar").into_string()
        });
        assert!(html.contains("Leaves"));
        let html = with_principal(auth(Superuser::NAME), registry, || {
            hr_menu("people").expect("sidebar").into_string()
        });
        assert!(html.contains("/hr/leaves"));
    }
}
