use crate::html_form::{
    Upload, html_form,
    widgets::{Duration, Email, File, Phone, Select, Text, Textarea},
};

use crate::plugins::filesystem::routes::VNodeFileSelectRouteTag;
use crate::plugins::forms::routes::FormFkSelectRouteTag;
use crate::plugins::forms::routes::FormResponseFkSelectRouteTag;
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

#[html_form]
pub struct ApplicantForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Mobile", required, widget = Phone)]
    pub mobile: String,

    #[form(label = "Email", required, widget = Email)]
    pub email: String,

    #[form(label = "Age", widget = Duration)]
    pub age: String,

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
        label = "Form response",
        widget = ForeignKey,
        route = FormResponseFkSelectRouteTag,
        swap_key = "hr-applicant-form-response",
        display = "form_response",
        placeholder = "Select form response…"
    )]
    pub form_response_id: String,

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

/// Confirm-only start probation modal.
#[html_form]
pub struct StartProbationForm {}

#[derive(Debug, Default, serde::Deserialize)]
pub struct StartProbationBody {}

/// Confirm-only hire modal.
#[html_form]
pub struct HireEmployeeForm {}

#[derive(Debug, Default, serde::Deserialize)]
pub struct HireEmployeeBody {}

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

    #[form(
        label = "Application form",
        required,
        widget = ForeignKey,
        route = FormFkSelectRouteTag,
        swap_key = "hr-job-form",
        display = "form",
        placeholder = "Select form…"
    )]
    pub form_id: i64,
}

#[html_form]
pub struct JobApplicationForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Email", required, widget = Email)]
    pub email: String,

    #[form(label = "Phone", required, widget = Phone)]
    pub mobile: String,

    #[form(label = "Age", widget = Duration)]
    pub age: String,

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
