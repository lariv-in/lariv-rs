use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonClear, ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldText,
        FormOpts, LayoutMain, LayoutSidebar, ObjectList, PaginationPage, ShellChrome,
        ShellScaffold, ShellTopbar, SidebarMenu, SidebarMenuItem, SlotCapability, SlotRegistrar,
        SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, app_layout_pane,
        button_clear, button_modal_form, button_submit, column_sort_url, container_column,
        container_row, data_table_list_refresh, delete_confirmation, detail, detail_header,
        field_text, form, form_hx_get_route, form_hx_post_multipart_url, form_hx_post_selector,
        form_hx_post_url, form_post_multipart, label, layout_main, layout_sidebar, modal,
        modal_keyed, pagination_pages, row_attr_navigate, shell_scaffold, shell_topbar,
        sidebar_menu, sidebar_menu_item_pane, sort_indicator, table_button_filter,
        table_pagination, with_list_filter_common,
    },
    html_form::{CsrfToken, FormCtx, HtmlForm},
    http::ProvideRequestCaps,
    template::{RenderAppPane, RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar},
    web::modal_create_post_url,
};

use super::crumbs::{applicant_crumbs, employee_crumbs, ex_employee_crumbs, hub_crumbs};
use super::detail_menu::{applicant_detail_menu, employee_detail_menu, ex_employee_detail_menu};
use super::forms::{
    ApplicantFilterForm, ApplicantFilterFormField, ApplicantForm, ApplicantFormField, EmployeeForm,
    EmployeeFormField, EmployeeFormFlag, HireApplicantForm, PersonForm, PersonFormField,
    TerminateEmployeeForm,
};
use super::keys::{
    ApplicantCreateModalKey, ApplicantDeleteModalKey, ApplicantEditModalKey, ApplicantHubTableKey,
    EmployeeCreateModalKey, EmployeeDeleteModalKey, ExEmployeeCreateModalKey,
    HireApplicantModalKey, TerminateEmployeeModalKey,
};
use super::logic::dashboard::MissingHrProfile;
use super::logic::profile::EmployeeProfileView;
use super::routes::{
    ApplicantCreateGetRouteTag, ApplicantCreatePostRouteTag, ApplicantDeleteGetRouteTag,
    ApplicantEditGetRouteTag, ApplicantEditPostRouteTag, ApplicantHubRouteTag,
    AttendanceListRouteTag, EmployeeCreateGetRouteTag, EmployeeCreatePostRouteTag,
    EmployeeDeleteGetRouteTag, EmployeeEditGetRouteTag, EmployeeEditPostRouteTag,
    ExEmployeeCreateGetRouteTag, ExEmployeeCreatePostRouteTag, HireApplicantGetRouteTag,
    HireApplicantPostRouteTag, HolidayListRouteTag, JobFormListRouteTag,
    HrDashboardPostRouteTag, TerminateEmployeeGetRouteTag, TerminateEmployeePostRouteTag,
};

pub(crate) fn app_scaffold(
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

pub(crate) fn scaffold_pane(
    sidebar: Option<Markup>,
    crumbs: Markup,
    body: Markup,
) -> crate::components::AppLayoutHtml {
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

pub(crate) fn scaffold_main(crumbs: Markup, body: Markup) -> crate::components::MainContentHtml {
    layout_main(LayoutMain {
        breadcrumbs: crumbs,
        content: body,
    })
}

/// HR sidebar. Each item follows that section's view allowlist.
pub fn hr_menu(active: &str) -> Option<Markup> {
    use crate::components::authorized_role;
    use crate::plugins::users::role_authorization::roles_for;

    use super::routes::{AttendanceView, HolidayView, HrPeopleView, JobFormView};

    let children = html! {
        (authorized_role(&roles_for::<HrPeopleView>(), html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "People",
                url: &ApplicantHubRouteTag.url(),
                active: active == "people",
                ..Default::default()
            }))
        }))
        (authorized_role(&roles_for::<JobFormView>(), html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Job postings",
                url: &JobFormListRouteTag.url(),
                active: active == "job-forms",
                ..Default::default()
            }))
        }))
        (authorized_role(&roles_for::<HolidayView>(), html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Holidays",
                url: &HolidayListRouteTag.url(),
                active: active == "holidays",
                ..Default::default()
            }))
        }))
        (authorized_role(&roles_for::<AttendanceView>(), html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Attendance",
                url: &AttendanceListRouteTag.url(),
                active: active == "attendance",
                ..Default::default()
            }))
        }))
    };
    let raw = children.into_string();
    if raw.trim().is_empty() {
        None
    } else {
        Some(sidebar_menu(SidebarMenu {
            title: "HR",
            children: maud::PreEscaped(raw),
        }))
    }
}

fn detail_sidebar(menu: Markup, roles: &[String]) -> Option<Markup> {
    let gated = crate::components::authorized_role(roles, menu);
    let raw = gated.into_string();
    if raw.trim().is_empty() {
        None
    } else {
        Some(maud::PreEscaped(raw))
    }
}

pub mod attendances;
pub mod holidays;
pub mod job_forms;

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
    pub aadhar_display: String,
    pub pan_display: String,
    pub passport_display: String,
    pub account_holder_name: String,
    pub account_number: String,
    pub account_ifsc_code: String,
    pub account_type: String,
    pub qualifications: String,
    pub date_of_joining: String,
    pub probation_end_date: String,
    pub work_start: String,
    pub work_end: String,
    pub base_salary: String,
    pub hourly_wage: String,
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
        .value(EmployeeFormField::DateOfJoining, &values.date_of_joining)
        .value(
            EmployeeFormField::ProbationEndDate,
            &values.probation_end_date,
        )
        .value(EmployeeFormField::WorkStart, &values.work_start)
        .value(EmployeeFormField::WorkEnd, &values.work_end)
        .value(EmployeeFormField::BaseSalary, &values.base_salary)
        .value(EmployeeFormField::HourlyWage, &values.hourly_wage)
        .choices(EmployeeFormField::Gender, &gender)
        .choices(EmployeeFormField::MaritalStatus, &marital)
        .choices(EmployeeFormField::Nationality, &nationality)
        .choices(EmployeeFormField::DisabilityType, &disability)
        .choices(EmployeeFormField::BloodGroup, &blood)
        .choices(EmployeeFormField::AccountType, &account);
    if include_admin_dates {
        ctx = ctx.flag(EmployeeFormFlag::AdminDates, true);
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

    let rendered = EmployeeForm::render_inputs(&ctx);

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
        (label("Aadhar", linked_value(&profile.aadhar_href, &profile.aadhar_label)))
        (label("PAN", linked_value(&profile.pan_href, &profile.pan_label)))
        (label("Passport", linked_value(&profile.passport_href, &profile.passport_label)))
        (label("Bank account holder name", field_text(FieldText { value: &profile.account_holder_name, classes: "" })))
        (label("Bank account number", field_text(FieldText { value: &profile.account_number, classes: "" })))
        (label("Bank account IFSC code", field_text(FieldText { value: &profile.account_ifsc_code, classes: "" })))
        (label("Bank account type", field_text(FieldText { value: &profile.account_type, classes: "" })))
        (label("Qualifications", field_text(FieldText { value: &profile.qualifications, classes: "" })))
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
        ApplicantEditModalIdx: ApplicantEditModalPageTag => ApplicantEditModalPage,
        PersonCreateModalIdx: PersonCreateModalPageTag => PersonCreateModalPage,
        PersonEditModalIdx: PersonEditModalPageTag => PersonEditModalPage,
        HireApplicantModalIdx: HireApplicantModalPageTag => HireApplicantModalPage,
        TerminateEmployeeModalIdx: TerminateEmployeeModalPageTag => TerminateEmployeeModalPage,
        EmployeeDetailIdx: EmployeeDetailPageTag => EmployeeDetailPage,
        ExEmployeeDetailIdx: ExEmployeeDetailPageTag => ExEmployeeDetailPage,
        ConfirmDeleteIdx: HrConfirmDeletePageTag => ConfirmDeletePage,
        JobFormListIdx: JobFormListPageTag => job_forms::JobFormListPage,
        JobFormSelectIdx: JobFormSelectPageTag => job_forms::JobFormSelectPage,
        JobFormDetailIdx: JobFormDetailPageTag => job_forms::JobFormDetailPage,
        JobFormCreateModalIdx: JobFormCreateModalPageTag => job_forms::JobFormCreateModalPage,
        JobFormEditModalIdx: JobFormEditModalPageTag => job_forms::JobFormEditModalPage,
        JobFormDeleteModalIdx: JobFormDeleteModalPageTag => job_forms::JobFormDeleteModalPage,
        HolidayListIdx: HolidayListPageTag => holidays::HolidayListPage,
        HolidayDetailIdx: HolidayDetailPageTag => holidays::HolidayDetailPage,
        HolidayCreateModalIdx: HolidayCreateModalPageTag => holidays::HolidayCreateModalPage,
        HolidayEditModalIdx: HolidayEditModalPageTag => holidays::HolidayEditModalPage,
        HolidayDeleteModalIdx: HolidayDeleteModalPageTag => holidays::HolidayDeleteModalPage,
        AttendanceListIdx: AttendanceListPageTag => attendances::AttendanceListPage,
        AttendanceDetailIdx: AttendanceDetailPageTag => attendances::AttendanceDetailPage,
        AttendanceCreateModalIdx: AttendanceCreateModalPageTag => attendances::AttendanceCreateModalPage,
        AttendanceEditModalIdx: AttendanceEditModalPageTag => attendances::AttendanceEditModalPage,
        AttendanceDeleteModalIdx: AttendanceDeleteModalPageTag => attendances::AttendanceDeleteModalPage,
        HrDashboardGateIdx: HrDashboardGatePageTag => HrDashboardGatePage,
        HrDashboardSuccessIdx: HrDashboardSuccessPageTag => HrDashboardSuccessPage,
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
        if crate::components::role_permitted(
            &crate::plugins::users::role_authorization::roles_for::<super::routes::ApplicantMutate>(
            ),
        ) {
            let create_button = match self.tab.as_str() {
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
    pub values: ApplicantFormValues,
    pub job_form_href: String,
    pub answers: Vec<crate::plugins::hr::questions::RenderedAnswer>,
    pub resume_href: String,
}

impl ApplicantDetailPage {
    fn body(&self) -> Markup {
        let actions = if crate::components::role_permitted(
            &crate::plugins::users::role_authorization::roles_for::<super::routes::ApplicantMutate>(
            ),
        ) {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.HireApplicantForm",
                    href: &HireApplicantGetRouteTag::new(self.id).url(),
                    form_post_url: &HireApplicantGetRouteTag::new(self.id).path(),
                    modal_uid: HireApplicantModalKey::ID,
                    label: "Hire as employee",
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
        let gender_label = crate::plugins::hr::gender::ApplicantGender::parse(&self.values.gender)
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(
            detail_sidebar(
                applicant_detail_menu(&self.display_name, self.id, "detail"),
                &crate::plugins::users::role_authorization::roles_for::<
                    super::routes::ApplicantMutate,
                >(),
            ),
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
            detail_sidebar(
                applicant_detail_menu(&self.display_name, self.id, "detail"),
                &crate::plugins::users::role_authorization::roles_for::<
                    super::routes::ApplicantMutate,
                >(),
            ),
            applicant_crumbs(&self.display_name),
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
    pub is_probationary: bool,
    pub profile: EmployeeProfileView,
}

impl EmployeeDetailPage {
    fn status_label(&self) -> &'static str {
        if self.is_probationary {
            "Started at"
        } else {
            "Hired at"
        }
    }

    fn probationary_label(&self) -> &'static str {
        if self.is_probationary { "Yes" } else { "No" }
    }

    fn body(&self) -> Markup {
        let actions = if crate::components::role_permitted(
            &crate::plugins::users::role_authorization::roles_for::<super::routes::EmployeeMutate>(
            ),
        ) {
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
        let status_label = self.status_label();
        let probationary_label = self.probationary_label();
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.display_name,
                        actions,
                    }))
                    (label(status_label, field_text(FieldText { value: &self.hired_at, classes: "" })))
                    (label("Probationary", field_text(FieldText { value: probationary_label, classes: "" })))
                    (person_fields(&self.name, &self.mobile, &self.email))
                    (employee_profile_fields(&self.profile))
                }))
            }))
        }
    }

    fn menu(&self) -> Option<Markup> {
        detail_sidebar(
            employee_detail_menu(&self.display_name, self.id, "detail"),
            &crate::plugins::users::role_authorization::roles_for::<super::routes::EmployeeMutate>(
            ),
        )
    }

    fn crumbs(&self) -> Markup {
        employee_crumbs(&self.display_name)
    }
}

impl RenderAppPane for EmployeeDetailPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        scaffold_pane(self.menu(), self.crumbs(), self.body())
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.body())
    }
}

impl RenderTemplate for EmployeeDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let title = if self.is_probationary {
            "Probation — Lariv"
        } else {
            "Employee — Lariv"
        };
        app_scaffold(title, chrome, self.menu(), self.crumbs(), self.body())
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
            detail_sidebar(
                ex_employee_detail_menu(&self.display_name, self.id, "detail"),
                &crate::plugins::users::role_authorization::roles_for::<
                    super::routes::ExEmployeeMutate,
                >(),
            ),
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
            detail_sidebar(
                ex_employee_detail_menu(&self.display_name, self.id, "detail"),
                &crate::plugins::users::role_authorization::roles_for::<
                    super::routes::ExEmployeeMutate,
                >(),
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
            employee: EmployeeFormValues::default(),
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
                        form_hx_post_multipart_url::<EmployeeCreateModalKey>(&post_url)
                    }
                    PersonCreateKind::ExEmployee => {
                        form_hx_post_url::<ExEmployeeCreateModalKey>(&post_url)
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
    pub values: ApplicantFormValues,
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
        modal_keyed::<ApplicantEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit applicant" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<ApplicantEditModalKey>(&self.post_url),
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
                            modal_uid: ApplicantDeleteModalKey::ID,
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
                    modal_uid: EmployeeDeleteModalKey::ID,
                    classes: "btn-error",
                    ..Default::default()
                }))
            };
        }
        modal_keyed::<ApplicantEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit employee" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_multipart_url::<ApplicantEditModalKey>(&self.post_url),
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
pub struct HireApplicantModalPage {
    pub applicant_id: i64,
    pub form_name: String,
    pub refresh_table: String,
    pub error: String,
}

impl RenderTemplate for HireApplicantModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<HireApplicantModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Hire applicant" }
                p class="mb-4 text-sm opacity-80" {
                    "Move this applicant into employees?"
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<HireApplicantModalKey>(&modal_create_post_url(
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
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", self.modal_uid);
        modal(crate::components::Modal {
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
            MissingHrProfile::Probation => "Probationary employee details",
            MissingHrProfile::Employee => "Employee details",
            MissingHrProfile::ExEmployee => "Ex-employee details",
        }
    }

    fn submit_label(&self) -> &'static str {
        match self.kind {
            MissingHrProfile::Applicant => "Submit applicant details",
            MissingHrProfile::Probation => "Submit probation details",
            MissingHrProfile::Employee => "Submit employee details",
            MissingHrProfile::ExEmployee => "Submit ex-employee details",
        }
    }

    fn body(&self) -> Markup {
        let inputs = match self.kind {
            MissingHrProfile::Applicant => applicant_form_inputs(&self.applicant),
            MissingHrProfile::Probation | MissingHrProfile::Employee => {
                employee_form_inputs(&self.employee, false)
            }
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        app_layout_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
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
                            (crate::components::text::icon("check-circle", "w-12 h-12"))
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
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        app_layout_pane(self.body())
    }

    fn render_main(&self) -> crate::components::MainContentHtml {
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
