use crate::html_form::{
    html_form,
    widgets::{Email, Phone, Text, Textarea},
};

use crate::plugins::forms::routes::FormFkSelectRouteTag;

#[html_form]
pub struct ApplicantForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Mobile", required, widget = Phone)]
    pub mobile: String,

    #[form(label = "Email", required, widget = Email)]
    pub email: String,
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

#[derive(Debug, serde::Deserialize)]
pub struct JobApplicationBody {
    pub name: String,
    pub email: String,
    pub mobile: String,
    pub answers_json: String,
}
