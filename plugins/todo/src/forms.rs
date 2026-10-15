//! Request form structs for todo items.

use lariv_core::html_form::{
    html_form,
    widgets::{Checkbox, Text, Textarea},
};

#[html_form]
pub struct TodoForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Description", widget = Textarea, rows = 4)]
    pub description: String,

    #[form(label = "Checked", widget = Checkbox)]
    pub checked: bool,
}
