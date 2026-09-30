use crate::html_form::{
    html_form,
    widgets::{Date, Select, Text, Textarea},
};
use crate::plugins::filesystem::routes::VNodeFileSelectRouteTag;

use super::document_type::DocumentType;
use super::gender::Gender;

#[html_form]
pub struct DocumentForm {
    #[form(label = "Document type", required, widget = Select, choices = "document_type")]
    pub document_type: String,

    #[form(
        label = "Aadhar card file",
        required,
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "fk-document-aadhar-file",
        display = "file",
        placeholder = "Select Aadhar card file…"
    )]
    pub vnode_id: i64,

    #[form(label = "Aadhar number", required, widget = Text)]
    pub aadhar_number: String,

    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Gender", required, widget = Select, choices = "gender")]
    pub gender: String,

    #[form(label = "Date of birth", required, widget = Date)]
    pub date_of_birth: String,

    #[form(label = "Address", required, widget = Textarea, rows = 4)]
    pub address: String,
}

impl DocumentForm {
    pub fn document_type_choices() -> &'static [(&'static str, &'static str)] {
        DocumentType::choices()
    }

    pub fn gender_choices() -> &'static [(&'static str, &'static str)] {
        Gender::choices()
    }
}

#[html_form]
pub struct DocumentFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}
