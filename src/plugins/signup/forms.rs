//! Request form structs for public signup.

use crate::html_form::{
    html_form,
    widgets::{Checkbox, Email, Password, Phone, Select, Text},
};

#[html_form]
pub struct SignupForm {
    #[form(label = "Full Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Email, required)]
    pub email: String,

    #[form(label = "Phone Number", widget = Phone, required)]
    pub phone: String,

    #[form(label = "Timezone", widget = Select, choices = "timezone", required)]
    pub timezone: String,

    #[form(label = "Password", widget = Password, required, name = "password1")]
    pub password1: String,

    #[form(label = "Confirm Password", widget = Password, required, name = "password2")]
    pub password2: String,

    #[form(label = "I accept the terms and conditions", widget = Checkbox, name = "terms_accepted")]
    pub terms_accepted: Option<String>,
}
