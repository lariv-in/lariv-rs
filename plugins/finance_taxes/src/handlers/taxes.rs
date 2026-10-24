use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_plugin_users::role_authorization::scope_allowed;
use rust_decimal::Decimal;
use sea_orm::sea_query::Condition;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseBackend, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};

use lariv_core::components::FilterClauseView;
use lariv_core::list_filters::{
    FilterField, FilterRowError, FilterWidget, apply_filters, custom, parse_list_filters,
    text_column,
};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::picker::respond_picker_select;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done_fk, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    entities::tax::{self, Entity as TaxEntity, TaxKind},
    forms::{TaxForm, tax_type_label},
    handlers::ModalNameQuery,
    keys::{
        TaxCreateModalKey, TaxDeleteModalKey, TaxEditModalKey, TaxMultiSelectModalKey,
        TaxMultiSelectTableKey, TaxTableKey,
    },
    routes::{TaxDefaultRouteTag, TaxDetailRouteTag},
    scope::{find_tax_scoped, model_to_row},
    state::TaxesState,
    templates::{
        ConfirmDeletePage, TaxCreateModalPage, TaxDetailPage, TaxEditModalPage, TaxListPage,
        TaxMultiSelectPage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaxListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

struct TaxListFilters {
    fields: Vec<FilterField>,
    filters: Vec<FilterClauseView>,
    filter_errors: Vec<FilterRowError>,
    condition: Condition,
}

fn tax_filter_fields() -> Vec<FilterField> {
    let choices = crate::forms::tax_type_choices();
    let listed = choices.clone();
    vec![
        text_column("name", "Name", tax::Column::Name),
        custom(
            "tax_type",
            "Type",
            FilterWidget::Choice,
            choices,
            false,
            move |clause, _| {
                let raw = clause.value();
                if !listed.iter().any(|(key, _)| key == raw) {
                    return Err(format!("unknown value: {raw}"));
                }
                let kind = TaxKind::parse(raw).ok_or_else(|| format!("unknown value: {raw}"))?;
                Ok(Condition::all().add(tax::Column::TaxType.eq(kind)))
            },
        ),
    ]
}

fn tax_list_filters(query: &str, backend: DatabaseBackend) -> TaxListFilters {
    let fields = tax_filter_fields();
    let clauses = parse_list_filters(query);
    let applied = apply_filters(backend, &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    TaxListFilters {
        fields,
        filters,
        filter_errors: applied.errors,
        condition: applied.condition,
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaxSelectQuery {
    #[serde(flatten)]
    pub filter: TaxListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn parse_percentage(s: &str) -> Option<Decimal> {
    lariv_plugin_finance_common::decimal::parse_decimal(s)
}

async fn journal_display(db: &sea_orm::DatabaseConnection, raw: &str) -> String {
    let id = raw.trim().parse::<i64>().ok().filter(|id| *id > 0);
    let label = lariv_plugin_finance_accounts::scope::load_journal_display_label(db, id).await;
    if label == "—" { String::new() } else { label }
}

async fn template_fields(
    db: &sea_orm::DatabaseConnection,
    tax_id: i64,
) -> (String, String, String) {
    let Ok(Some(template)) = crate::journal_template::load_template(db, tax_id).await else {
        let (journal_id, journal_display) =
            lariv_plugin_finance_accounts::scope::load_default_journal_form(db).await;
        return (
            journal_id,
            journal_display,
            crate::journal_template::empty_lines_json(),
        );
    };
    let mut labels = Vec::with_capacity(template.items.len());
    for item in &template.items {
        let label = lariv_plugin_finance_accounts::scope::load_account_parent_label(
            db,
            Some(item.account_id),
        )
        .await;
        labels.push(if label == "—" { String::new() } else { label });
    }
    (
        template.journal_id.to_string(),
        journal_display(db, &template.journal_id.to_string()).await,
        crate::journal_template::lines_to_json(&template.items, &labels),
    )
}

async fn query_taxes(
    db: &sea_orm::DatabaseConnection,
    query_string: &str,
    sort: Option<&str>,
    page: u32,
    page_size: u32,
) -> (ObjectList<crate::templates::TaxRow>, TaxListFilters) {
    let filters = tax_list_filters(query_string, db.get_database_backend());
    let mut query = scope_allowed::<super::super::routes::FinanceTaxesView, _>(TaxEntity::find());
    query = query.filter(filters.condition.clone());

    let sort = sort.unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(tax::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(tax::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Type DESC") => query.order_by_desc(tax::Column::TaxType),
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(tax::Column::TaxType)
        }
        s if s.eq_ignore_ascii_case("Percentage DESC") => {
            query.order_by_desc(tax::Column::Percentage)
        }
        s if s.eq_ignore_ascii_case("Percentage ASC") || s.eq_ignore_ascii_case("Percentage") => {
            query.order_by_asc(tax::Column::Percentage)
        }
        _ => query.order_by_desc(tax::Column::Id),
    };
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for model in models {
        rows.push(model_to_row(db, model).await);
    }
    (ObjectList::from_page(rows, page, page_size, total), filters)
}

pub async fn list(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TaxListQuery>,
) -> maud::Markup {
    let (taxes, filters) = query_taxes(
        &state.db,
        uri.query().unwrap_or(""),
        q.sort.as_deref(),
        q.page.get(),
        q.page_size.get(),
    )
    .await;
    let page = TaxListPage {
        taxes,
        fields: filters.fields,
        filters: filters.filters,
        filter_errors: filters.filter_errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<TaxTableKey>() {
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

pub async fn detail(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(t) = find_tax_scoped(&state.db, id).await else {
        return Redirect::to(&TaxDefaultRouteTag.url()).into_response();
    };
    let (journal_name, journal_lines) =
        match crate::journal_template::load_template(&state.db, t.id).await {
            Ok(Some(template)) => {
                let journal_name =
                    lariv_plugin_finance_accounts::scope::load_journal_display_label(
                        &state.db,
                        Some(template.journal_id),
                    )
                    .await;
                let mut lines = Vec::new();
                for item in &template.items {
                    let account = lariv_plugin_finance_accounts::scope::load_account_parent_label(
                        &state.db,
                        Some(item.account_id),
                    )
                    .await;
                    lines.push(format!("{account}: {}", item.formula));
                }
                (journal_name, lines.join("\n"))
            }
            _ => ("—".into(), "—".into()),
        };
    let page = TaxDetailPage {
        id: t.id,
        name: t.name,
        tax_type: tax_type_label(&t.tax_type),
        percentage: t.percentage.normalize().to_string(),
        journal_name,
        journal_lines,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let (journal_id, journal_display) =
        lariv_plugin_finance_accounts::scope::load_default_journal_form(&state.db).await;
    let page = TaxCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        tax_type: TaxKind::Levied.as_str().to_string(),
        percentage: String::new(),
        journal_id,
        journal_display,
        journal_lines: crate::journal_template::empty_lines_json(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TaxForm>,
) -> Response {
    let journal_display = journal_display(&state.db, &form.journal_id).await;
    let render_error = |error: String| {
        let page = TaxCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            target_input: q.target_input(),
            name: form.name.clone(),
            tax_type: form.tax_type.clone(),
            percentage: form.percentage.clone(),
            journal_id: form.journal_id.clone(),
            journal_display: journal_display.clone(),
            journal_lines: form.journal_lines.clone(),
            error,
        };
        html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
    };
    let Some(tax_type) = TaxKind::parse(&form.tax_type) else {
        return render_error("Invalid tax type".into());
    };
    let Some(percentage) = parse_percentage(&form.percentage) else {
        return render_error("Invalid percentage".into());
    };
    let now = Utc::now();
    let model = tax::ActiveModel {
        name: Set(form.name.clone()),
        tax_type: Set(tax_type),
        percentage: Set(percentage),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    let saved = match model.insert(&state.db).await {
        Ok(saved) => saved,
        Err(e) => return render_error(e.to_string()),
    };
    if let Err(e) = crate::journal_template::save_tax_template(
        &state.db,
        saved.id,
        &form.journal_id,
        &form.journal_lines,
    )
    .await
    {
        let _ = TaxEntity::delete_by_id(saved.id).exec(&state.db).await;
        return render_error(e);
    }
    respond_create_modal_done_fk::<TaxCreateModalKey>(
        &htmx,
        &q.refresh_table(),
        &TaxDetailRouteTag::new(saved.id).url(),
        saved.id,
        &saved.name,
        &q.target_input(),
    )
}

pub async fn edit_get(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(t) = find_tax_scoped(&state.db, id).await else {
        return Redirect::to(&TaxDefaultRouteTag.url()).into_response();
    };
    let (journal_id, journal_display, journal_lines) = template_fields(&state.db, t.id).await;
    let page = TaxEditModalPage {
        id: t.id,
        form_name: q.form_name(),
        name: t.name,
        tax_type: t.tax_type.as_str().to_string(),
        percentage: t.percentage.normalize().to_string(),
        journal_id,
        journal_display,
        journal_lines,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn tax_edit_modal_page_from_form(
    id: i64,
    form: &TaxForm,
    form_name: String,
    journal_display: String,
    error: String,
) -> TaxEditModalPage {
    TaxEditModalPage {
        id,
        form_name,
        name: form.name.clone(),
        tax_type: form.tax_type.clone(),
        percentage: form.percentage.clone(),
        journal_id: form.journal_id.clone(),
        journal_display,
        journal_lines: form.journal_lines.clone(),
        error,
    }
}

pub async fn edit_post(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<TaxForm>,
) -> Response {
    let Some(existing) = find_tax_scoped(&state.db, id).await else {
        return Redirect::to(&TaxDefaultRouteTag.url()).into_response();
    };
    let journal_display = journal_display(&state.db, &form.journal_id).await;
    let Some(tax_type) = TaxKind::parse(&form.tax_type) else {
        let page = tax_edit_modal_page_from_form(
            id,
            &form,
            q.form_name(),
            journal_display,
            "invalid tax type".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    let Some(percentage) = parse_percentage(&form.percentage) else {
        let page = tax_edit_modal_page_from_form(
            id,
            &form,
            q.form_name(),
            journal_display,
            "invalid percentage".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    let now = Utc::now();
    let model = tax::ActiveModel {
        id: Set(existing.id),
        name: Set(form.name.clone()),
        tax_type: Set(tax_type),
        percentage: Set(percentage),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    if let Err(e) = model.update(&state.db).await {
        let page =
            tax_edit_modal_page_from_form(id, &form, q.form_name(), journal_display, e.to_string());
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    if let Err(e) = crate::journal_template::save_tax_template(
        &state.db,
        id,
        &form.journal_id,
        &form.journal_lines,
    )
    .await
    {
        let page = tax_edit_modal_page_from_form(id, &form, q.form_name(), journal_display, e);
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    respond_edit_modal_done::<TaxEditModalKey>(&htmx, &TaxDetailRouteTag::new(id).url())
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: TaxDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this tax?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_finance_taxes.TaxDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if find_tax_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&TaxDefaultRouteTag.url()).into_response();
    }
    match TaxEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&TaxDefaultRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete tax");
            let page = ConfirmDeletePage {
                modal_uid: TaxDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this tax?".into(),
                form_name: "p_finance_taxes.TaxDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn multi_select(
    Cap(state): Cap<TaxesState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TaxSelectQuery>,
) -> maud::Markup {
    let (taxes, filters) = query_taxes(
        &state.db,
        uri.query().unwrap_or(""),
        q.filter.sort.as_deref(),
        q.filter.page.get(),
        q.filter.page_size.get(),
    )
    .await;
    let page = TaxMultiSelectPage {
        taxes,
        fields: filters.fields,
        filters: filters.filters,
        filter_errors: filters.filter_errors,
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "TaxIds".into()),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select::<TaxMultiSelectTableKey, TaxMultiSelectModalKey, _>(&htmx, &page)
}
