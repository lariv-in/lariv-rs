use lariv_core::html_form::html_form;
use lariv_plugin_contacts::routes::{CompanyFkSelectRouteTag, ContactFkSelectRouteTag};

/// A customer is one company or one contact. Exactly one of the two is set.
#[html_form]
pub struct CustomerForm {
    #[form(
        label = "Company",
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "customer-company",
        display = "company",
        placeholder = "Select company…"
    )]
    pub company_id: i64,

    #[form(
        label = "Contact",
        widget = ForeignKey,
        route = ContactFkSelectRouteTag,
        swap_key = "customer-contact",
        display = "individual",
        placeholder = "Select contact…"
    )]
    pub individual_id: i64,
}
