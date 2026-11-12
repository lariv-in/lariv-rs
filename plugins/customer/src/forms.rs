use lariv_core::html_form::{html_form, widgets::Select};
use lariv_plugin_contacts::routes::{CompanyFkSelectRouteTag, ContactFkSelectRouteTag};

use crate::gst_state::GST_STATE_CHOICES;

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

    #[form(label = "State", widget = Select, choices = "state")]
    pub state: String,
}

impl CustomerForm {
    pub fn state_choices() -> &'static [(&'static str, &'static str)] {
        GST_STATE_CHOICES
    }
}

#[cfg(test)]
mod tests {
    use super::{CustomerForm, CustomerFormField};
    use crate::gst_state::GST_STATE_CHOICES;
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn state_select_lists_gst_codes() {
        let html = CustomerForm::render_inputs(
            &FormCtx::form::<CustomerForm>(CsrfToken::current())
                .choices(CustomerFormField::State, GST_STATE_CHOICES),
        )
        .into_string();
        assert!(html.contains("27 — MAHARASHTRA"), "{html}");
        assert!(!html.contains("value=\"25\""), "{html}");
        assert!(!html.contains("value=\"28\""), "{html}");
    }
}
