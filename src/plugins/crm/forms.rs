use crate::html_form::{
    html_form,
    widgets::{Checkbox, Date, Datetime, Select, Text, Textarea},
};
use crate::plugins::users::routes::UsersSelectRouteTag;

use super::lead_source::LeadSource;
use super::routes::{CompanyFkSelectRouteTag, ContactFkSelectRouteTag};

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

#[html_form]
pub struct ContactForm {
    #[form(
        label = "Company",
        required,
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "crm-contact-company",
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
        swap_key = "crm-contact-filter-company",
        display = "company",
        placeholder = "Any company…"
    )]
    pub company_id: String,

    #[form(label = "Name", widget = Text)]
    pub name: String,
}

#[html_form]
pub struct TaskForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Description", widget = Textarea)]
    pub description: String,

    #[form(
        label = "Assigned To",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "crm-task-assigned-to",
        display = "assigned_to",
        placeholder = "Select user…"
    )]
    pub assigned_to_id: i64,

    #[form(label = "Due Date", widget = Date)]
    pub due_date: String,
}

#[html_form]
pub struct TaskFilterForm {
    #[form(label = "Title", widget = Text)]
    pub title: String,

    #[form(
        label = "Assigned To",
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "crm-task-filter-assigned-to",
        display = "assigned_to",
        placeholder = "Any user…"
    )]
    pub assigned_to_id: String,
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

#[cfg(test)]
mod tests {
    use super::ConvertLeadForm;
    use crate::html_form::{FormCtx, HtmlForm};

    #[test]
    fn convert_lead_form_is_empty() {
        let html =
            ConvertLeadForm::render_inputs(&FormCtx::form::<ConvertLeadForm>()).into_string();
        assert!(!html.contains("name=\"DealKind\""), "{html}");
        assert!(!html.contains("name=\"DealName\""), "{html}");
        assert!(!html.contains("name=\"CreateDeal\""), "{html}");
    }
}
