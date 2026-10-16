//! Request form for creating and editing a to-do page.

use lariv_core::html_form::{html_form, widgets::Text};

#[html_form]
pub struct TodoPageForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,
}
