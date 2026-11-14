use lariv_core::html_form::{
    html_form,
    widgets::{Checkbox, Select, Text},
};

use crate::gst_state::GST_STATE_CHOICES;

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

    #[form(label = "State", widget = Select, choices = "state")]
    pub state: String,

    #[form(label = "Website", widget = Text)]
    pub website: String,

    #[form(label = "GSTIN", widget = Text)]
    pub gstin: String,

    #[form(label = "CIN", widget = Text)]
    pub cin: String,

    #[form(label = "PAN", widget = Text)]
    pub pan: String,

    #[form(label = "Phone", widget = Text)]
    pub phone: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,
}

impl CompanyForm {
    pub fn state_choices() -> &'static [(&'static str, &'static str)] {
        GST_STATE_CHOICES
    }
}

#[cfg(test)]
mod tests {
    use super::{CompanyForm, CompanyFormField};
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn state_select_lists_gst_codes() {
        let html = CompanyForm::render_inputs(
            &FormCtx::form::<CompanyForm>(CsrfToken::current())
                .choices(CompanyFormField::State, CompanyForm::state_choices()),
        )
        .into_string();
        assert!(html.contains("<select"), "{html}");
        assert!(html.contains("27 — MAHARASHTRA"), "{html}");
        assert!(html.contains("value=\"27\""), "{html}");
        assert!(!html.contains("value=\"25\""), "{html}");
    }
}
