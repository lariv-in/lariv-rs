use axum::extract::Path;
use axum::response::{IntoResponse, Redirect, Response};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::datetime::format_optional_date;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_contacts::scope::contact_display_label;
use lariv_plugin_users::lookup::user_display_label;
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    entities::{
        converted_lead::{self, Entity as ConvertedLeadEntity},
        failed_lead::{self, Entity as FailedLeadEntity},
    },
    handlers::lead_updates::load_updates_panel,
    keys::LEAD_UPDATES,
    routes::{ConvertedLeadDetailRouteTag, FailedLeadDetailRouteTag, LeadDefaultRouteTag},
    scope::{
        company_display_label, find_active_lead, find_converted_lead_scoped,
        find_failed_lead_scoped, find_lead_scoped, lead_contact_view,
    },
    state::CrmState,
    templates::{LeadConvertDetailPage, LeadDetailPage, LeadFailDetailPage},
};

use super::{lead_tag_chips, source_label};

pub async fn detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(lead) = find_lead_scoped(&state.db, id).await else {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    };
    if find_active_lead(&state.db, id).await.is_none() {
        if let Some(c) = lariv_core::web::opt_or_log(
            ConvertedLeadEntity::find()
                .filter(converted_lead::Column::LeadId.eq(id))
                .one(&state.db)
                .await,
            "db find one",
        ) {
            return Redirect::to(&ConvertedLeadDetailRouteTag::new(c.id).url()).into_response();
        }
        if let Some(f) = lariv_core::web::opt_or_log(
            FailedLeadEntity::find()
                .filter(failed_lead::Column::LeadId.eq(id))
                .one(&state.db)
                .await,
            "db find one",
        ) {
            return Redirect::to(&FailedLeadDetailRouteTag::new(f.id).url()).into_response();
        }
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    }
    let view = lead_contact_view(&state.db, lead.contact_id).await;
    let page = LeadDetailPage {
        id: lead.id,
        display_name: if view.display_name.is_empty() {
            format!("Lead #{}", lead.id)
        } else {
            view.display_name
        },
        contact_id: view.contact_id,
        contact_display: contact_display_label(&state.db, view.contact_id).await,
        company_id: view.company_id,
        company: view.company,
        email: view.email,
        source: source_label(lead.source),
        notes: lead.notes.unwrap_or_default(),
        assigned_to: user_display_label(&state.db, lead.assigned_to_id.unwrap_or(0)).await,
        order_expected_date: format_optional_date(lead.order_expected_date),
        tags: lead_tag_chips(&state.db, lead.id).await,
        updates: load_updates_panel(&state.db, &ctx, lead.id).await,
    };
    if htmx.targets(LEAD_UPDATES) {
        return page.updates.render_list().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn converted_detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(converted) = find_converted_lead_scoped(&state.db, id).await else {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    };
    let lead = find_lead_scoped(&state.db, converted.lead_id).await;
    let view = match &lead {
        Some(l) => lead_contact_view(&state.db, l.contact_id).await,
        None => Default::default(),
    };
    let display_name = if view.display_name.is_empty() {
        format!("Lead #{}", converted.lead_id)
    } else {
        view.display_name.clone()
    };
    let page = LeadConvertDetailPage {
        converted_id: converted.id,
        lead_id: converted.lead_id,
        display_name,
        converted_at: converted
            .converted_at
            .format("%d/%m/%Y %H:%M UTC")
            .to_string(),
        company_id: converted.company_id,
        contact_id: converted.contact_id,
        company: company_display_label(&state.db, converted.company_id).await,
        contact_display: contact_display_label(&state.db, converted.contact_id).await,
        email: view.email,
        source: lead
            .as_ref()
            .map(|l| source_label(l.source))
            .unwrap_or_default(),
        notes: lead
            .as_ref()
            .and_then(|l| l.notes.clone())
            .unwrap_or_default(),
        assigned_to: user_display_label(
            &state.db,
            lead.as_ref().and_then(|l| l.assigned_to_id).unwrap_or(0),
        )
        .await,
        order_expected_date: format_optional_date(
            lead.as_ref().and_then(|l| l.order_expected_date),
        ),
        tags: lead_tag_chips(&state.db, converted.lead_id).await,
        updates: load_updates_panel(&state.db, &ctx, converted.lead_id).await,
    };
    if htmx.targets(LEAD_UPDATES) {
        return page.updates.render_list().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn failed_detail(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(failed) = find_failed_lead_scoped(&state.db, id).await else {
        return Redirect::to(&LeadDefaultRouteTag.url()).into_response();
    };
    let lead = find_lead_scoped(&state.db, failed.lead_id).await;
    let view = match &lead {
        Some(l) => lead_contact_view(&state.db, l.contact_id).await,
        None => Default::default(),
    };
    let display_name = if view.display_name.is_empty() {
        format!("Lead #{}", failed.lead_id)
    } else {
        view.display_name.clone()
    };
    let page = LeadFailDetailPage {
        failed_id: failed.id,
        lead_id: failed.lead_id,
        display_name,
        failed_at: failed.failed_at.format("%d/%m/%Y %H:%M UTC").to_string(),
        reason: failed.reason.unwrap_or_default(),
        contact_id: view.contact_id,
        contact_display: contact_display_label(&state.db, view.contact_id).await,
        company_id: view.company_id,
        company: view.company,
        email: view.email,
        source: lead
            .as_ref()
            .map(|l| source_label(l.source))
            .unwrap_or_default(),
        notes: lead
            .as_ref()
            .and_then(|l| l.notes.clone())
            .unwrap_or_default(),
        assigned_to: user_display_label(
            &state.db,
            lead.as_ref().and_then(|l| l.assigned_to_id).unwrap_or(0),
        )
        .await,
        order_expected_date: format_optional_date(
            lead.as_ref().and_then(|l| l.order_expected_date),
        ),
        tags: lead_tag_chips(&state.db, failed.lead_id).await,
        updates: load_updates_panel(&state.db, &ctx, failed.lead_id).await,
    };
    if htmx.targets(LEAD_UPDATES) {
        return page.updates.render_list().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}
