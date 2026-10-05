use lariv_core::html_form::{
    Upload, html_form,
    widgets::{
        Checkbox, Date, Datetime, Decimal, Email, File, Phone, Select, SingleChoiceCombobox, Text,
        Textarea, Time,
    },
};

use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;
use lariv_plugin_forms::forms::FormQuestionsDraft;
use crate::blood_group::BloodGroup;
use crate::entities::leaves::LeaveType;
use crate::gender::ApplicantGender;
use crate::logic::leave::{
    FILTER_APPROVED, FILTER_PENDING, FILTER_REJECTED, STATUS_APPROVED, STATUS_PENDING,
    STATUS_REJECTED,
};
use crate::routes::JobFormFkSelectRouteTag;
use lariv_plugin_users::routes::UsersSelectRouteTag;

#[html_form]
pub struct PersonForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Mobile", required, widget = Phone)]
    pub mobile: String,

    #[form(label = "Email", required, widget = Email)]
    pub email: String,
}

pub const MARITAL_STATUS_CHOICES: &[(&str, &str)] = &[
    ("single", "Single"),
    ("married", "Married"),
    ("divorced", "Divorced"),
    ("widowed", "Widowed"),
    ("separated", "Separated"),
];

pub const ACCOUNT_TYPE_CHOICES: &[(&str, &str)] = &[
    ("savings", "Savings"),
    ("current", "Current"),
    ("salary", "Salary"),
];

#[html_form]
pub struct EmployeeForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Mobile", required, widget = Phone)]
    pub mobile: String,

    #[form(label = "Email", required, widget = Email)]
    pub email: String,

    #[form(label = "Father's name", widget = Text)]
    pub fathers_name: String,

    #[form(label = "Date of birth", widget = Date)]
    pub date_of_birth: String,

    #[form(label = "Gender", widget = Select, choices = "gender")]
    pub gender: String,

    #[form(label = "Marital status", widget = Select, choices = "marital_status")]
    pub marital_status: String,

    #[form(
        label = "Nationality",
        widget = SingleChoiceCombobox,
        choices = "nationality",
        placeholder = "Search country…"
    )]
    pub nationality: String,

    #[form(label = "Is disabled", widget = Checkbox, model = "is_disabled")]
    pub is_disabled: bool,

    #[form(
        label = "Disability type",
        widget = SingleChoiceCombobox,
        choices = "disability_type",
        show = "is_disabled",
        placeholder = "Search disability…"
    )]
    pub disability_type: String,

    #[form(label = "Photograph", widget = File, accept = "image/*")]
    pub photograph: Option<Upload>,

    #[form(label = "Blood group", widget = Select, choices = "blood_group")]
    pub blood_group: String,

    #[form(label = "Identification mark", widget = Text)]
    pub identification_mark: String,

    #[form(label = "Present address", widget = Textarea, rows = 3, model = "present_address")]
    pub present_address: String,

    #[form(label = "Present PIN code", widget = Text, model = "present_pin_code")]
    pub present_pin_code: String,

    #[form(
        label = "Permanent address same as present address",
        widget = Checkbox,
        model = "same_as_present"
    )]
    pub same_as_present: bool,

    #[form(
        label = "Permanent address",
        widget = Textarea,
        rows = 3,
        model = "permanent_address",
        disabled = "same_as_present"
    )]
    pub permanent_address: String,

    #[form(
        label = "Permanent PIN code",
        widget = Text,
        model = "permanent_pin_code",
        disabled = "same_as_present"
    )]
    pub permanent_pin_code: String,

    #[form(label = "Emergency contact name", widget = Text)]
    pub emergency_contact_name: String,

    #[form(label = "Emergency contact relation", widget = Text)]
    pub emergency_contact_relation: String,

    #[form(label = "Emergency contact mobile", widget = Phone)]
    pub emergency_contact_mobile: String,

    #[form(label = "Aadhar", widget = File, accept = ".pdf,.jpg,.jpeg,.png")]
    pub aadhar: Option<Upload>,

    #[form(label = "PAN", widget = File, accept = ".pdf,.jpg,.jpeg,.png")]
    pub pan: Option<Upload>,

    #[form(label = "Passport", widget = File, accept = ".pdf,.jpg,.jpeg,.png")]
    pub passport: Option<Upload>,

    #[form(label = "Bank account holder name", widget = Text)]
    pub account_holder_name: String,

    #[form(label = "Bank account number", widget = Text)]
    pub account_number: String,

    #[form(label = "Bank account IFSC code", widget = Text)]
    pub account_ifsc_code: String,

    #[form(label = "Bank account type", widget = Select, choices = "account_type")]
    pub account_type: String,

    #[form(label = "Qualifications", widget = Textarea, rows = 4)]
    pub qualifications: String,

    /// No preselected manager. Required only when the employee is a superuser
    /// (`manager_optional` is off). Hidden when the actor is editing their own record.
    #[form(
        label = "Manager",
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-employee-manager",
        display = "manager",
        placeholder = "Select manager…",
        when = "edit_manager",
        required_unless = "manager_optional"
    )]
    pub manager_id: Option<i64>,

    #[form(label = "Date of joining", widget = Date, when = "admin_dates")]
    pub date_of_joining: String,

    #[form(label = "Probation end date", widget = Date, when = "admin_dates")]
    pub probation_end_date: String,

    #[form(label = "Work start", widget = Time, when = "admin_dates")]
    pub work_start: String,

    #[form(label = "Work end", widget = Time, when = "admin_dates")]
    pub work_end: String,

    #[form(label = "Base salary", widget = Decimal, when = "admin_dates")]
    pub base_salary: String,

    #[form(label = "Hourly wage", widget = Decimal, when = "admin_dates")]
    pub hourly_wage: String,
}

impl EmployeeForm {
    pub fn gender_choices() -> &'static [(&'static str, &'static str)] {
        ApplicantGender::choices()
    }

    pub fn marital_status_choices() -> &'static [(&'static str, &'static str)] {
        MARITAL_STATUS_CHOICES
    }

    pub fn nationality_choices() -> &'static [(&'static str, &'static str)] {
        crate::countries::ALL_COUNTRIES
    }

    pub fn disability_type_choices() -> &'static [(&'static str, &'static str)] {
        crate::disability::REGISTERED_DISABILITIES_INDIA
    }

    pub fn blood_group_choices() -> &'static [(&'static str, &'static str)] {
        BloodGroup::choices()
    }

    pub fn account_type_choices() -> &'static [(&'static str, &'static str)] {
        ACCOUNT_TYPE_CHOICES
    }
}

#[html_form]
pub struct ApplicantForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Mobile", required, widget = Phone)]
    pub mobile: String,

    #[form(label = "Email", required, widget = Email)]
    pub email: String,

    #[form(label = "Date of birth", widget = Datetime)]
    pub date_of_birth: String,

    #[form(label = "Gender", widget = Select, choices = "gender")]
    pub gender: String,

    #[form(label = "Address", widget = Textarea, rows = 3)]
    pub address: String,

    #[form(label = "Remarks", widget = Textarea, rows = 4)]
    pub remarks: String,

    #[form(
        label = "Job posting",
        widget = ForeignKey,
        route = JobFormFkSelectRouteTag,
        swap_key = "hr-applicant-job-form",
        display = "job_form",
        placeholder = "Select job posting…"
    )]
    pub job_form_id: String,

    #[form(
        label = "Resume",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "hr-applicant-resume",
        display = "resume",
        placeholder = "Select resume file…"
    )]
    pub resume_vnode_id: String,
}

impl ApplicantForm {
    pub fn gender_choices() -> &'static [(&'static str, &'static str)] {
        ApplicantGender::choices()
    }
}

#[html_form]
pub struct ApplicantFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,
}

/// Confirm hiring an applicant as an employee.
#[html_form]
pub struct HireApplicantForm {}

#[derive(Debug, Default, serde::Deserialize)]
pub struct HireApplicantBody {}

/// Confirm-only terminate modal.
#[html_form]
pub struct TerminateEmployeeForm {}

#[derive(Debug, Default, serde::Deserialize)]
pub struct TerminateEmployeeBody {}

#[html_form]
pub struct JobFormForm {
    #[form(label = "Job title", required, widget = Text)]
    pub job_title: String,

    #[form(label = "Salary range", widget = Text)]
    pub salary_range: String,

    #[form(label = "Experience required", widget = Text)]
    pub experience_required: String,

    #[form(label = "Description", required, widget = Textarea, rows = 6)]
    pub description: String,

    #[form(label = "Questions", widget = FormQuestionsDraft)]
    pub questions_json: String,
}

#[html_form]
pub struct JobApplicationForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Email", required, widget = Email)]
    pub email: String,

    #[form(label = "Phone", required, widget = Phone)]
    pub mobile: String,

    #[form(label = "Date of birth", widget = Datetime)]
    pub date_of_birth: String,

    #[form(label = "Gender", widget = Select, choices = "gender")]
    pub gender: String,

    #[form(label = "Address", widget = Textarea, rows = 3)]
    pub address: String,

    #[form(label = "Remarks", widget = Textarea, rows = 4)]
    pub remarks: String,

    #[form(label = "Resume", widget = File, accept = ".pdf,.doc,.docx,.odt,.rtf,.txt")]
    pub resume: Option<Upload>,

    #[form(name = "answers_json", label = "Answers", widget = Textarea)]
    pub answers_json: String,
}

#[html_form]
pub struct HolidayForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Description", widget = Textarea, rows = 4)]
    pub description: String,

    #[form(label = "Date", required, widget = Date)]
    pub date: String,
}

#[html_form]
pub struct HolidayFilterForm {
    #[form(label = "Title", widget = Text)]
    pub title: String,

    #[form(label = "Date", widget = Date)]
    pub date: String,
}

#[html_form]
pub struct AttendanceForm {
    #[form(
        label = "User",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-attendance-user",
        display = "user",
        placeholder = "Select user…"
    )]
    pub user_id: i64,

    #[form(label = "Start", required, widget = Datetime)]
    pub started_at: String,

    #[form(label = "End", required, widget = Datetime)]
    pub ended_at: String,
}

#[html_form]
pub struct AttendanceFilterForm {
    #[form(
        label = "User",
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-attendance-filter-user",
        display = "user",
        placeholder = "Any user…"
    )]
    pub user_id: String,

    #[form(label = "Start", widget = Datetime)]
    pub started_at: String,

    #[form(label = "End", widget = Datetime)]
    pub ended_at: String,
}

#[html_form]
pub struct LeaveApplicationForm {
    #[form(label = "Date", required, widget = Date)]
    pub date: String,

    #[form(label = "Type", required, widget = Select, choices = "leave_type")]
    pub leave_type: String,

    #[form(label = "Reason", required, widget = Textarea, rows = 4)]
    pub reason: String,
}

impl LeaveApplicationForm {
    pub fn leave_type_choices() -> &'static [(&'static str, &'static str)] {
        LeaveType::choices()
    }
}

#[html_form]
pub struct LeaveApplicationFilterForm {
    #[form(
        label = "Applied by",
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-leave-filter-applied-by",
        display = "applied_by",
        placeholder = "Any user…"
    )]
    pub applied_by_id: String,

    #[form(label = "Date", widget = Date)]
    pub date: String,

    #[form(label = "Type", widget = Select, choices = "leave_type")]
    pub leave_type: String,

    #[form(label = "Status", widget = Select, choices = "status")]
    pub status: String,

    #[form(label = "Reason", widget = Text)]
    pub reason: String,
}

impl LeaveApplicationFilterForm {
    pub fn leave_type_choices() -> &'static [(&'static str, &'static str)] {
        LeaveType::choices()
    }

    pub fn status_choices() -> &'static [(&'static str, &'static str)] {
        &[
            (FILTER_PENDING, STATUS_PENDING),
            (FILTER_APPROVED, STATUS_APPROVED),
            (FILTER_REJECTED, STATUS_REJECTED),
        ]
    }
}

/// Confirm-only. The approver is the signed-in user and the time is now.
#[html_form]
pub struct ApproveLeaveForm {}

#[html_form]
pub struct RejectLeaveForm {
    #[form(label = "Reason", widget = Textarea, rows = 3)]
    pub reason: String,
}

/// Confirm-only. Removes the approval and returns the leave to pending.
#[html_form]
pub struct RevokeApprovalForm {}

/// Confirm-only. Removes the rejection and returns the leave to pending.
#[html_form]
pub struct RevokeRejectionForm {}
