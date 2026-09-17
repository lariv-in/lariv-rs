use crate::html_form::{
    html_form,
    widgets::{Checkbox, Text},
};

use super::routes::CompanyFkSelectRouteTag;

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

#[html_form]
pub struct ContactFilterForm {
    #[form(
        label = "Company",
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "contact-filter-company",
        display = "company",
        placeholder = "Any company…"
    )]
    pub company_id: String,

    #[form(label = "Name", widget = Text)]
    pub name: String,
}

#[html_form]
pub struct CompanyForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Address line 1", widget = Text)]
    pub address_line_1: String,

    #[form(label = "Address line 2", widget = Text)]
    pub address_line_2: String,

    #[form(label = "City", widget = Text)]
    pub city: String,

    #[form(label = "Pincode", widget = Text)]
    pub pincode: String,

    #[form(label = "State", widget = Text)]
    pub state: String,

    #[form(label = "Website", widget = Text)]
    pub website: String,
}

#[html_form]
pub struct CompanyFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}
