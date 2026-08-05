//! Request form structs for users.

use crate::html_form::{
    html_form,
    widgets::{Checkbox, Email, ForeignKey, Password, Phone, Select, Text},
};

#[html_form]
pub struct LoginForm {
    #[form(label = "Email", widget = Email, required)]
    pub email: String,

    #[form(label = "Password", widget = Password, required)]
    pub password: String,
}

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
        widget = ForeignKey,
        url = "/users/roles/select/",
        swap_key = "fk-user-role",
        display = "role",
        required,
        placeholder = "Select a role..."
    )]
    pub role_id: i64,
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
pub struct RoleForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,
}

#[html_form]
pub struct UserFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Email)]
    pub email: String,

    #[form(label = "Phone", widget = Phone)]
    pub phone: String,
}

#[html_form]
pub struct UserSelectFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Email)]
    pub email: String,
}

#[html_form]
pub struct RoleNameFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}
