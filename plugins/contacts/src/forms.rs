use lariv_core::html_form::{
    html_form,
    widgets::{Checkbox, Text},
};

use lariv_plugin_companies::routes::CompanyFkSelectRouteTag;

#[html_form]
pub struct ContactForm {
    #[form(
        label = "Company",
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "contact-company",
        display = "company",
        placeholder = "Select company…"
    )]
    pub company_id: i64,

    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,

    #[form(label = "Phone", widget = Text)]
    pub phone: String,

    #[form(label = "Primary contact", widget = Checkbox)]
    pub is_primary: String,
}
