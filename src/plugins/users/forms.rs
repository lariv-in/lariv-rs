//! Request form structs for users.

use crate::html_form::{
    html_form,
    widgets::{Email, Password, Phone, Role, Select, Text},
};

#[html_form]
pub struct LoginForm {
    #[form(label = "Email", widget = Email, required)]
    pub email: String,

    #[form(label = "Password", widget = Password, required)]
    pub password: String,
}

#[html_form]
pub struct UserForm {
    #[form(label = "Name", required, widget = Text, row = "identity")]
    pub name: String,

    #[form(label = "Email", widget = Email, required, row = "identity")]
    pub email: String,

    #[form(label = "Phone", widget = Phone, required)]
    pub phone: String,

    #[form(label = "Timezone", widget = Select, choices = "timezone", required)]
    pub timezone: String,

    #[form(
        label = "Role",
        widget = Role,
        required,
        placeholder = "Select a role..."
    )]
    pub role: String,
}

#[html_form]
pub struct SelfEditForm {
    #[form(label = "Name", required, widget = Text, row = "identity")]
    pub name: String,

    #[form(label = "Email", widget = Email, required, row = "identity")]
    pub email: String,

    #[form(label = "Phone", widget = Phone, required)]
    pub phone: String,

    #[form(label = "Timezone", widget = Select, choices = "timezone", required)]
    pub timezone: String,
}

#[html_form]
pub struct PasswordForm {
    #[form(label = "New Password", widget = Password, required, name = "new_password")]
    pub new_password: String,

    #[form(
        label = "Confirm New Password",
        widget = Password,
        required,
        name = "confirm_password"
    )]
    pub confirm_password: String,
}

#[html_form]
pub struct UserFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,

    #[form(label = "Phone", widget = Phone)]
    pub phone: String,
}

#[html_form]
pub struct UserSelectFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,
}
