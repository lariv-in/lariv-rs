use crate::html_form::{
    html_form,
    widgets::{Checkbox, Date, Text},
};

#[html_form]
pub struct FiscalYearForm {
    #[form(label = "Code", required, widget = Text)]
    pub code: String,

    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Start", required, widget = Date)]
    pub start: String,

    #[form(label = "End", required, widget = Date)]
    pub end: String,

    #[form(label = "Active", widget = Checkbox)]
    pub is_active: bool,
}

#[html_form]
pub struct FiscalYearFilterForm {
    #[form(label = "Code", widget = Text)]
    pub code: String,

    #[form(label = "Name", widget = Text)]
    pub name: String,
}
