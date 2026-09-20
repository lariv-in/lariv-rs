use crate::html_form::{
    html_form,
    widgets::{Checkbox, Datetime, Email, Text},
};

#[html_form]
pub struct CreateRoomForm {
    #[form(label = "Allow anonymous guests", widget = Checkbox)]
    pub anonymous_allowed: bool,

    #[form(label = "Allow new people to join", widget = Checkbox)]
    pub joining_allowed: bool,

    #[form(label = "Scheduled start (optional)", widget = Datetime)]
    pub start_at: String,
}

#[html_form]
pub struct CreateRoomFilterForm {
    #[form(label = "Code", widget = Text)]
    pub code: String,
}

#[html_form]
pub struct AnonymousJoinForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Email", required, widget = Email)]
    pub email: String,
}
