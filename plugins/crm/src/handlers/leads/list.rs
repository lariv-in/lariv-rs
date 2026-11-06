use axum::http::Uri;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::{Condition, IntoColumnRef};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::db::trigram::text_match_condition;
use lariv_core::html_form::UrlencodedFields;
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    FilterClause, FilterField, FilterWidget, apply_filters, custom, many_to_many, parse_id_list,
    parse_list_filters,
};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_with_slots};
use lariv_plugin_contacts::entities::contact;
use lariv_plugin_contacts::routes::CompanyFkSelectRouteTag;
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    entities::{
        converted_lead::{self, Entity as ConvertedLeadEntity},
        failed_lead::{self, Entity as FailedLeadEntity},
        lead::{self, Entity as LeadEntity},
    },
    handlers::lead_tags::tag_items_from_ids,
    keys::LEAD_HUB_TABLE,
    routes::{
        ConvertedLeadDetailRouteTag, FailedLeadDetailRouteTag, LeadDetailRouteTag,
        LeadTagSelectRouteTag,
    },
    scope::{
        any_lead_tag_condition, apply_converted_lead_sort, apply_failed_lead_sort, apply_lead_sort,
        apply_lead_tag_id_filter, company_display_label, join_active_lead_relations,
        join_converted_lead_relations, join_failed_lead_relations, lead_active_condition,
        lead_contact_view,
    },
    state::CrmState,
    templates::{LeadHubPage, LeadRow},
};

use super::source_label;

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct HubQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub sort: Option<String>,
}

fn hub_query_from_uri(uri: &Uri) -> HubQuery {
    let Some(query) = uri.query() else {
        return HubQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

fn lead_filter_fields<C>(lead_id_col: C) -> Vec<FilterField>
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    vec![
        custom(
            "company_id",
            "Company",
            FilterWidget::ForeignKey {
                url: format!("{}?filter.0.field=name", CompanyFkSelectRouteTag.url()),
                placeholder: "Any company…".into(),
                search_key: "filter.0.value".into(),
            },
            Vec::new(),
            false,
            |clause, _| {
                let raw = clause.value();
                let id = raw
                    .parse::<i64>()
                    .map_err(|_| format!("invalid id: {raw}"))?;
                if id <= 0 {
                    return Err(format!("invalid id: {raw}"));
                }
                Ok(Condition::all().add(contact::Column::CompanyId.eq(id)))
            },
        ),
        custom(
            "contact",
            "Contact",
            FilterWidget::Text,
            Vec::new(),
            false,
            |clause, backend| {
                let needle = clause.value().trim();
                if needle.is_empty() {
                    return Ok(Condition::all());
                }
                Ok(text_match_condition(
                    backend,
                    &[contact::Column::Name, contact::Column::Email],
                    needle,
                ))
            },
        ),
        many_to_many(
            "tags",
            "Tags",
            LeadTagSelectRouteTag.url(),
            "Any tags…",
            move |clause, _| {
                let ids = parse_id_list(clause)?;
                Ok(any_lead_tag_condition(lead_id_col, &ids))
            },
        ),
    ]
}

fn needs_contact_join(clauses: &[FilterClause]) -> bool {
    clauses
        .iter()
        .any(|clause| clause.field == "company_id" || clause.field == "contact")
}

async fn fill_lead_filter_views(db: &sea_orm::DatabaseConnection, rows: &mut [FilterClauseView]) {
    for row in rows {
        if row.clause.field == "company_id" {
            let Ok(id) = row.clause.value().parse::<i64>() else {
                continue;
            };
            if id > 0 {
                row.fk_display = company_display_label(db, id).await;
            }
        } else if row.clause.field == "tags" {
            if let Ok(ids) = parse_id_list(&row.clause) {
                row.m2m_items = tag_items_from_ids(db, &ids).await;
            }
        }
    }
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub(crate) async fn query_active_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
    condition: Condition,
    needs_contact: bool,
    forced_tag_ids: &[i64],
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = scope_allowed::<crate::routes::CrmView, _>(LeadEntity::find())
        .filter(lead_active_condition());
    query = join_active_lead_relations(query, q.sort.as_deref(), needs_contact);
    query = query.filter(condition);
    query = apply_lead_tag_id_filter(query, lead::Column::Id, forced_tag_ids);
    query = apply_lead_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for l in models {
        let view = lead_contact_view(db, l.contact_id).await;
        rows.push(LeadRow {
            id: l.id,
            name: if view.display_name.is_empty() {
                format!("Lead #{}", l.id)
            } else {
                view.display_name
            },
            company: view.company,
            email: view.email,
            source: source_label(l.source),
            status: "Active".to_string(),
            detail_href: LeadDetailRouteTag::new(l.id).url(),
        });
    }
    (rows, page_num, total)
}

pub(crate) async fn query_converted_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
    condition: Condition,
    needs_contact: bool,
    forced_tag_ids: &[i64],
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = scope_allowed::<crate::routes::CrmView, _>(ConvertedLeadEntity::find());
    query = join_converted_lead_relations(query, q.sort.as_deref(), needs_contact);
    query = query.filter(condition);
    query = apply_lead_tag_id_filter(query, converted_lead::Column::LeadId, forced_tag_ids);
    let query = apply_converted_lead_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let converted = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::new();
    for c in converted {
        let lead = lariv_core::web::opt_or_log(
            LeadEntity::find_by_id(c.lead_id).one(db).await,
            "find by id",
        );
        let (name, company, email, source) = match lead {
            Some(l) => {
                let view = lead_contact_view(db, l.contact_id).await;
                (
                    if view.display_name.is_empty() {
                        format!("Lead #{}", l.id)
                    } else {
                        view.display_name
                    },
                    view.company,
                    view.email,
                    source_label(l.source),
                )
            }
            None => (
                format!("Lead #{}", c.lead_id),
                String::new(),
                String::new(),
                String::new(),
            ),
        };
        rows.push(LeadRow {
            id: c.id,
            name,
            company,
            email,
            source,
            status: "Converted".to_string(),
            detail_href: ConvertedLeadDetailRouteTag::new(c.id).url(),
        });
    }
    (rows, page_num, total)
}

pub(crate) async fn query_failed_leads(
    db: &sea_orm::DatabaseConnection,
    q: &HubQuery,
    page_size: u32,
    condition: Condition,
    needs_contact: bool,
    forced_tag_ids: &[i64],
) -> (Vec<LeadRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = scope_allowed::<crate::routes::CrmView, _>(FailedLeadEntity::find());
    query = join_failed_lead_relations(query, q.sort.as_deref(), needs_contact);
    query = query.filter(condition);
    query = apply_lead_tag_id_filter(query, failed_lead::Column::LeadId, forced_tag_ids);
    query = apply_failed_lead_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let failed = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::new();
    for f in failed {
        let lead = lariv_core::web::opt_or_log(
            LeadEntity::find_by_id(f.lead_id).one(db).await,
            "find by id",
        );
        let (name, company, email, source) = match lead {
            Some(l) => {
                let view = lead_contact_view(db, l.contact_id).await;
                (
                    if view.display_name.is_empty() {
                        format!("Lead #{}", l.id)
                    } else {
                        view.display_name
                    },
                    view.company,
                    view.email,
                    source_label(l.source),
                )
            }
            None => (
                format!("Lead #{}", f.lead_id),
                String::new(),
                String::new(),
                String::new(),
            ),
        };
        rows.push(LeadRow {
            id: f.id,
            name,
            company,
            email,
            source,
            status: "Failed".to_string(),
            detail_href: FailedLeadDetailRouteTag::new(f.id).url(),
        });
    }
    (rows, page_num, total)
}

pub async fn hub(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let q = hub_query_from_uri(&uri);
    let tab = q.tab.as_deref().unwrap_or("active").to_string();
    let clauses = parse_list_filters(uri.query().unwrap_or(""));
    let fields = match tab.as_str() {
        "converted" => lead_filter_fields(converted_lead::Column::LeadId),
        "failed" => lead_filter_fields(failed_lead::Column::LeadId),
        _ => lead_filter_fields(lead::Column::Id),
    };
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let needs_contact = needs_contact_join(&clauses);
    let (rows, page, total) = match tab.as_str() {
        "converted" => {
            query_converted_leads(
                &state.db,
                &q,
                q.page_size.get(),
                applied.condition,
                needs_contact,
                &[],
            )
            .await
        }
        "failed" => {
            query_failed_leads(
                &state.db,
                &q,
                q.page_size.get(),
                applied.condition,
                needs_contact,
                &[],
            )
            .await
        }
        _ => {
            query_active_leads(
                &state.db,
                &q,
                q.page_size.get(),
                applied.condition,
                needs_contact,
                &[],
            )
            .await
        }
    };
    let leads = ObjectList::from_page(rows, page, q.page_size.get(), total);
    let mut filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    fill_lead_filter_views(&state.db, &mut filters).await;
    let page = LeadHubPage {
        leads,
        tab,
        fields,
        filters,
        filter_errors: applied.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(LEAD_HUB_TABLE) {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}
