use crate::html_form::{
    html_form,
    widgets::{Email, Phone, Text},
};

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
