use crate::html_form::{
    empty_str_as_i64, html_form,
    widgets::{Checkbox, Kind, Select, Text, Textarea},
};

use super::deal_stage::DealStage;
use super::lead_source::LeadSource;
use super::routes::{CompanyFkSelectRouteTag, ContactFkSelectRouteTag};

const _: fn() = || {
    let _: Kind = Kind;
};

#[html_form]
pub struct LeadForm {
    #[form(label = "Company name", widget = Text)]
    pub company_name: String,

    #[form(label = "First name", widget = Text)]
    pub first_name: String,

    #[form(label = "Last name", widget = Text)]
    pub last_name: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,

    #[form(label = "Phone", widget = Text)]
    pub phone: String,

    #[form(label = "Source", required, widget = Select)]
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
    #[form(label = "Company", widget = Text)]
    pub company: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,
}

#[html_form(tag = "DealKind", default)]
pub enum ConvertLeadDealSource {
    #[form(label = "No deal")]
    None,

    #[form(label = "Create deal")]
    Create {
        #[form(label = "Deal name", widget = Text)]
        deal_name: String,

        #[form(label = "Deal amount", widget = Text)]
        deal_amount: String,

        #[form(label = "Deal stage", widget = Select)]
        deal_stage: String,
    },
}

impl ConvertLeadDealSource {
    pub fn deal_stage_choices() -> &'static [(&'static str, &'static str)] {
        DealStage::choices()
    }
}

#[html_form(default)]
pub struct ConvertLeadForm {
    #[form(
        label = "Company",
        required,
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "crm-convert-company",
        display = "company",
        placeholder = "Select company…"
    )]
    pub company_id: i64,

    #[form(widget = Kind)]
    pub deal_source: ConvertLeadDealSource,
}

/// Urlencoded POST body for convert (matches [`ConvertLeadForm`] field names).
#[derive(Debug, serde::Deserialize)]
pub struct ConvertLeadBody {
    #[serde(
        rename = "CompanyID",
        alias = "company_id",
        default,
        deserialize_with = "empty_str_as_i64"
    )]
    pub company_id: i64,

    #[serde(rename = "DealKind", alias = "deal_kind", default)]
    pub deal_kind: String,

    #[serde(rename = "DealName", alias = "deal_name", default)]
    pub deal_name: String,

    #[serde(rename = "DealAmount", alias = "deal_amount", default)]
    pub deal_amount: String,

    #[serde(rename = "DealStage", alias = "deal_stage", default)]
    pub deal_stage: String,
}

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

    #[form(label = "First name", required, widget = Text)]
    pub first_name: String,

    #[form(label = "Last name", widget = Text)]
    pub last_name: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,

    #[form(label = "Phone", widget = Text)]
    pub phone: String,

    #[form(label = "Title", widget = Text)]
    pub title: String,

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
pub struct DealForm {
    #[form(
        label = "Company",
        required,
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "crm-deal-company",
        display = "company",
        placeholder = "Select company…"
    )]
    pub company_id: i64,

    #[form(
        label = "Primary contact",
        required,
        widget = ForeignKey,
        route = ContactFkSelectRouteTag,
        swap_key = "crm-deal-contact",
        display = "primary_contact",
        placeholder = "Select contact…"
    )]
    pub primary_contact_id: i64,

    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Amount", widget = Text)]
    pub amount: String,

    #[form(label = "Stage", required, widget = Select)]
    pub stage: String,

    #[form(label = "Expected close date", widget = Text)]
    pub expected_close_date: String,
}

impl DealForm {
    pub fn stage_choices() -> &'static [(&'static str, &'static str)] {
        DealStage::choices()
    }
}

#[html_form]
pub struct DealFilterForm {
    #[form(
        label = "Company",
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "crm-deal-filter-company",
        display = "company",
        placeholder = "Any company…"
    )]
    pub company_id: String,

    #[form(label = "Name", widget = Text)]
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::{ConvertLeadDealSource, ConvertLeadForm, ConvertLeadDealSourceField, ConvertLeadFormField};
    use crate::html_form::{FormCtx, HtmlForm, HtmlKind};

    #[test]
    fn convert_lead_deal_kind_variants() {
        assert_eq!(ConvertLeadDealSource::kind_tag(), "DealKind");
        let v = ConvertLeadDealSource::variants();
        assert_eq!(v[0].value, "None");
        assert_eq!(v[1].value, "Create");
    }

    #[test]
    fn convert_lead_form_renders_company_and_deal_fields() {
        let ctx = FormCtx::form::<ConvertLeadForm>()
            .value(ConvertLeadFormField::CompanyId, "1")
            .display(ConvertLeadFormField::CompanyId, "Acme")
            .kind::<ConvertLeadDealSource>("Create")
            .value(ConvertLeadDealSourceField::DealName, "Opportunity");
        let html = ConvertLeadForm::render_inputs(&ctx).into_string();
        assert!(html.contains("name=\"CompanyID\""), "{html}");
        assert!(html.contains("name=\"DealKind\""), "{html}");
        assert!(html.contains("name=\"DealName\""), "{html}");
        assert!(!html.contains("name=\"CompanyKind\""), "{html}");
        assert!(!html.contains("name=\"CreateDeal\""), "{html}");
    }
}
