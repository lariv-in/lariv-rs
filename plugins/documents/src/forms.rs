use lariv_core::html_form::{
    html_form,
    widgets::{Date, Duration, Select, Text, Textarea},
};
use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;

use super::document_type::DocumentType;
use super::gender::Gender;

#[html_form]
pub struct DocumentForm {
    #[form(
        label = "Document type",
        required,
        widget = Select,
        choices = "document_type",
        when = "show_type_field"
    )]
    pub document_type: String,

    #[form(
        label = "Document file",
        required,
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "fk-document-file",
        display = "file",
        placeholder = "Select document file…"
    )]
    pub vnode_id: i64,

    #[form(label = "Aadhar number", required, widget = Text, when = "is_aadhar")]
    pub aadhar_number: String,

    #[form(label = "PAN", required, widget = Text, when = "is_pan")]
    pub pan_number: String,

    #[form(label = "Passport number", required, widget = Text, when = "is_passport")]
    pub passport_number: String,

    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Gender", required, widget = Select, choices = "gender", when = "has_gender")]
    pub gender: String,

    #[form(label = "Date of birth", required, widget = Date)]
    pub date_of_birth: String,

    #[form(label = "Address", required, widget = Textarea, rows = 4, when = "is_aadhar")]
    pub address: String,

    #[form(label = "Nationality", required, widget = Text, when = "is_passport")]
    pub nationality: String,

    #[form(label = "Expiry date", required, widget = Date, when = "is_passport")]
    pub expiry_date: String,
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
pub struct PreferencesForm {
    #[form(label = "Signing Authority Name", required, widget = Text)]
    pub signing_authority_name: String,

    #[form(label = "Validity Duration", required, widget = Duration)]
    pub validity_duration: String,
}

#[html_form]
pub struct DocumentFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}
