use lariv_core::components::ManyToManyItem;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    entities::{
        converted_lead::{self, Entity as ConvertedLeadEntity},
        failed_lead::{self, Entity as FailedLeadEntity},
    },
    forms::LeadForm,
    handlers::{ModalNameQuery, lead_tags::load_tags_for_lead},
    lead_source::LeadSource,
    logic::lead::LeadInput,
    routes::{ConvertedLeadDetailRouteTag, FailedLeadDetailRouteTag, LeadDetailRouteTag},
    templates::{ConvertLeadModalPage, LeadCreateModalPage, LeadTagChip},
};

pub mod create;
pub mod detail;
pub mod edit;
pub mod list;

pub use create::{create_get, create_post};
pub use detail::{converted_detail, detail, failed_detail};
pub use edit::{
    convert_get, convert_post, converted_reactivate_post, delete_get, delete_post, edit_get,
    edit_post, fail_get, fail_post, reactivate_post,
};
pub use list::hub;
pub(crate) use list::{HubQuery, query_active_leads, query_converted_leads, query_failed_leads};

pub(crate) fn opt_string(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

pub(crate) fn parse_lead_source(raw: &str) -> Option<LeadSource> {
    LeadSource::parse(raw)
}

pub(crate) fn source_label(source: Option<LeadSource>) -> String {
    source.map(|s| s.label().to_string()).unwrap_or_default()
}

pub(crate) fn source_value(source: Option<LeadSource>) -> String {
    source.map(|s| s.as_str().to_string()).unwrap_or_default()
}

pub(crate) fn convert_modal_page_from_body(
    lead_id: i64,
    q: &ModalNameQuery,
    error: String,
) -> ConvertLeadModalPage {
    ConvertLeadModalPage {
        lead_id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        error,
    }
}

pub(crate) fn parse_order_expected_date(
    s: &str,
) -> Result<Option<chrono::NaiveDate>, &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    lariv_core::datetime::parse_date(s)
        .map(Some)
        .ok_or("invalid order expected date")
}

pub(crate) fn lead_input_from_form(form: &LeadForm) -> Result<LeadInput, String> {
    let order_expected_date = parse_order_expected_date(&form.order_expected_date)?;
    Ok(LeadInput {
        customer_id: form.customer_id,
        source: parse_lead_source(&form.source),
        notes: opt_string(form.notes.clone()),
        assigned_to_id: (form.assigned_to_id > 0).then_some(form.assigned_to_id),
        order_expected_date,
        tag_ids: form.tags.clone(),
    })
}

pub(crate) async fn lead_tag_chips(
    db: &sea_orm::DatabaseConnection,
    lead_id: i64,
) -> Vec<LeadTagChip> {
    load_tags_for_lead(db, lead_id)
        .await
        .into_iter()
        .map(|t| LeadTagChip {
            id: t.id,
            name: t.name,
            color: t.color,
        })
        .collect()
}

pub(crate) fn lead_create_modal_page(
    q: &ModalNameQuery,
    customer_id: i64,
    customer_display: String,
    source: String,
    notes: String,
    tags: Vec<ManyToManyItem>,
    assigned_to_id: i64,
    assigned_to_display: String,
    order_expected_date: String,
    error: String,
) -> LeadCreateModalPage {
    LeadCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        customer_id,
        customer_display,
        source,
        notes,
        tags,
        assigned_to_id,
        assigned_to_display,
        order_expected_date,
        error,
    }
}

pub(crate) async fn lead_return_url(db: &sea_orm::DatabaseConnection, lead_id: i64) -> String {
    if let Some(c) = lariv_core::web::opt_or_log(
        ConvertedLeadEntity::find()
            .filter(converted_lead::Column::LeadId.eq(lead_id))
            .one(db)
            .await,
        "db find one",
    ) {
        return ConvertedLeadDetailRouteTag::new(c.id).url();
    }
    if let Some(f) = lariv_core::web::opt_or_log(
        FailedLeadEntity::find()
            .filter(failed_lead::Column::LeadId.eq(lead_id))
            .one(db)
            .await,
        "db find one",
    ) {
        return FailedLeadDetailRouteTag::new(f.id).url();
    }
    LeadDetailRouteTag::new(lead_id).url()
}
