use crate::html_form::{
    html_form,
    widgets::{Color, Date, Datetime, Select, Text, Textarea},
};
use crate::plugins::users::routes::UsersSelectRouteTag;

use crate::plugins::contacts::routes::{CompanyFkSelectRouteTag, ContactFkSelectRouteTag};

use super::lead_source::LeadSource;
use super::routes::LeadTagSelectRouteTag;

#[html_form]
pub struct LeadForm {
    #[form(
        label = "Contact",
        required,
        widget = ForeignKey,
        route = ContactFkSelectRouteTag,
        swap_key = "crm-lead-contact",
        display = "contact",
        placeholder = "Select contact…"
    )]
    pub contact_id: i64,

    #[form(label = "Source", widget = Select)]
    pub source: String,

    #[form(
        label = "Tags",
        widget = ManyToMany,
        route = LeadTagSelectRouteTag,
        swap_key = "crm-lead-tags",
        placeholder = "Select tags…"
    )]
    pub tags: Vec<i64>,

    #[form(
        label = "Salesperson",
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "crm-lead-assigned-to",
        display = "assigned_to",
        placeholder = "Select user…"
    )]
    pub assigned_to_id: i64,

    #[form(label = "Order expected date", widget = Date)]
    pub order_expected_date: String,

    #[form(label = "Notes", widget = Textarea)]
    pub notes: String,
}

/// POST body for lead edit — lead fields plus optional failure reason.
#[derive(Debug, serde::Deserialize)]
pub struct LeadEditBody {
    #[serde(flatten)]
    pub lead: LeadForm,

    #[serde(rename = "Reason", alias = "reason", default)]
    pub reason: String,
}

impl LeadForm {
    pub fn source_choices() -> &'static [(&'static str, &'static str)] {
        LeadSource::choices()
    }
}

#[html_form]
pub struct LeadTagForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Color", required, widget = Color)]
    pub color: String,
}

#[html_form]
pub struct LeadTagFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}

#[html_form]
pub struct LeadFilterForm {
    #[form(
        label = "Company",
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "crm-lead-filter-company",
        display = "company",
        placeholder = "Any company…"
    )]
    pub company_id: String,

    #[form(label = "Contact", widget = Text)]
    pub contact: String,

    #[form(
        label = "Tags",
        widget = ManyToMany,
        route = LeadTagSelectRouteTag,
        swap_key = "crm-lead-filter-tags",
        placeholder = "Any tags…"
    )]
    pub tags: Vec<i64>,
}

/// Confirm-only convert modal (no extra fields).
#[html_form]
pub struct ConvertLeadForm {}

/// Urlencoded POST body for convert (matches [`ConvertLeadForm`]; currently empty).
#[derive(Debug, Default, serde::Deserialize)]
pub struct ConvertLeadBody {}

#[html_form]
pub struct FailLeadForm {
    #[form(label = "Reason", widget = Textarea)]
    pub reason: String,
}

#[html_form]
pub struct LeadUpdateForm {
    #[form(
        label = "Created by",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "crm-lead-update-created-by",
        display = "created_by",
        placeholder = "Select user…"
    )]
    pub created_by_id: i64,

    #[form(label = "Date & time", required, widget = Datetime)]
    pub datetime: String,

    #[form(label = "Description", required, widget = Textarea)]
    pub description: String,
}

/// Inline add form on the lead detail page (datetime + description, localStorage draft).
#[html_form]
pub struct LeadUpdateQuickForm {
    #[form(label = "Date & time", required, widget = Datetime, model = "datetime")]
    pub datetime: String,

    #[form(label = "Description", required, widget = Textarea, model = "description")]
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::ConvertLeadForm;
    use crate::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn convert_lead_form_is_empty() {
        let html =
            ConvertLeadForm::render_inputs(&FormCtx::form::<ConvertLeadForm>(CsrfToken::current()))
                .into_string();
        assert!(!html.contains("name=\"DealKind\""), "{html}");
        assert!(!html.contains("name=\"DealName\""), "{html}");
        assert!(!html.contains("name=\"CreateDeal\""), "{html}");
    }
}
