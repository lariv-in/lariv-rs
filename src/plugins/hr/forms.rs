use crate::html_form::{
    Upload, html_form,
    widgets::{Checkbox, Date, Datetime, Email, File, Phone, Select, Text, Textarea},
};

use crate::plugins::documents::routes::{
    DocumentAadharSelectRouteTag, DocumentPanSelectRouteTag, DocumentPassportSelectRouteTag,
};

use crate::plugins::filesystem::routes::VNodeFileSelectRouteTag;
use crate::plugins::forms::forms::FormQuestionsDraft;
use crate::plugins::hr::blood_group::BloodGroup;
use crate::plugins::hr::gender::ApplicantGender;
use crate::plugins::hr::routes::JobFormFkSelectRouteTag;

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

    #[form(label = "Nationality", widget = Text)]
    pub nationality: String,

    #[form(label = "Is disabled", widget = Checkbox)]
    pub is_disabled: bool,

    #[form(label = "Disability type", widget = Text)]
    pub disability_type: String,

    #[form(
        label = "Photograph",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "hr-employee-photograph",
        display = "photograph",
        placeholder = "Select photograph…"
    )]
    pub photograph_vnode_id: String,

    #[form(label = "Blood group", widget = Select, choices = "blood_group")]
    pub blood_group: String,

    #[form(label = "Identification mark", widget = Text)]
    pub identification_mark: String,

    #[form(label = "Present address", widget = Textarea, rows = 3)]
    pub present_address: String,

    #[form(label = "Present PIN code", widget = Text)]
    pub present_pin_code: String,

    #[form(label = "Permanent address", widget = Textarea, rows = 3)]
    pub permanent_address: String,

    #[form(label = "Permanent PIN code", widget = Text)]
    pub permanent_pin_code: String,

    #[form(label = "Emergency contact name", widget = Text)]
    pub emergency_contact_name: String,

    #[form(label = "Emergency contact relation", widget = Text)]
    pub emergency_contact_relation: String,

    #[form(label = "Emergency contact mobile", widget = Phone)]
    pub emergency_contact_mobile: String,

    #[form(
        label = "Aadhar",
        widget = ForeignKey,
        route = DocumentAadharSelectRouteTag,
        swap_key = "hr-employee-aadhar",
        display = "aadhar",
        placeholder = "Select Aadhar…"
    )]
    pub aadhar_document_id: String,

    #[form(
        label = "PAN",
        widget = ForeignKey,
        route = DocumentPanSelectRouteTag,
        swap_key = "hr-employee-pan",
        display = "pan",
        placeholder = "Select PAN…"
    )]
    pub pan_document_id: String,

    #[form(
        label = "Passport",
        widget = ForeignKey,
        route = DocumentPassportSelectRouteTag,
        swap_key = "hr-employee-passport",
        display = "passport",
        placeholder = "Select passport…"
    )]
    pub passport_document_id: String,

    #[form(label = "Account holder name", widget = Text)]
    pub account_holder_name: String,

    #[form(label = "Account number", widget = Text)]
    pub account_number: String,

    #[form(label = "Account IFSC code", widget = Text)]
    pub account_ifsc_code: String,

    #[form(label = "Account type", widget = Select, choices = "account_type")]
    pub account_type: String,

    #[form(label = "Qualifications", widget = Textarea, rows = 4)]
    pub qualifications: String,

    #[form(label = "Date of joining", widget = Date)]
    pub date_of_joining: String,

    #[form(label = "Probation end date", widget = Date)]
    pub probation_end_date: String,
}

impl EmployeeForm {
    pub fn gender_choices() -> &'static [(&'static str, &'static str)] {
        ApplicantGender::choices()
    }

    pub fn marital_status_choices() -> &'static [(&'static str, &'static str)] {
        MARITAL_STATUS_CHOICES
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
