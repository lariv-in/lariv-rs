use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use lariv_core::components::{
    ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey, table_rows_instance_id,
};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::picker::respond_picker_select;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_create_modal_done_fk, respond_edit_modal_done,
};
use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::role_authorization::scope_allowed;

use crate::entities::stock_movement_in::{self, Entity as InEntity};
use crate::entities::stock_movement_process::{self, Entity as ProcessEntity};
use crate::entities::stock_movement_process_line::ProcessLineSide;
use crate::{
    forms::{MovementPartyForm, ProcessMovementForm},
    handlers::{ModalNameQuery, path_and_query},
    keys::{
        MovementCreateModalKey, MovementDeleteModalKey, MovementEditModalKey,
        MovementInSelectModalKey, MovementInSelectTableKey, MovementProcessSelectModalKey,
        MovementProcessSelectTableKey, MovementTableKey,
    },
    logic::{
        line::{
            lines_form_json, load_in_lines, load_out_lines, load_process_lines,
            parse_movement_lines_json,
        },
        movement::{
            PartyInput, ProcessInput, create_in, create_out, create_process, delete_in, delete_out,
            delete_process, out_sources_for, require_challan, require_out_sources, update_in,
            update_out, update_process,
        },
        party::{self},
        preferences::id_value,
        qty::{format_on_hand, type_label},
    },
    movement_type::MovementKind,
    routes::{InventoryView, MovementDefaultRouteTag, MovementDetailRouteTag},
    scope::{
        DEFAULT_MOVEMENT_SORT, effective_sort, find_movement_in, find_movement_out,
        find_movement_process, in_movement_labels, list_movements, page_of,
        process_movement_labels, slice_page, sort_movements, stock_names, user_names,
    },
    state::InventoryState,
    templates::{
        ConfirmDeletePage, MovementCreateModalPage, MovementDetailPage, MovementEditModalPage,
        MovementLineRow, MovementListPage, MovementRow, MovementSourceSelectPage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct MovementListQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn list_url() -> String {
    movement_tab_url(MovementKind::In)
}

fn movement_tab_url(kind: MovementKind) -> String {
    lariv_core::http::RouteQueryBuilder::new(MovementDefaultRouteTag)
        .query("tab", kind.as_str())
        .build()
}

fn detail_url(kind: MovementKind, id: i64) -> String {
    MovementDetailRouteTag::new(kind.as_str().to_string(), id).url()
}

pub async fn list(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<MovementListQuery>,
) -> maud::Markup {
    let kind = q
        .tab
        .as_deref()
        .and_then(MovementKind::parse)
        .unwrap_or(MovementKind::In);
    let sort = effective_sort(q.sort.as_deref(), DEFAULT_MOVEMENT_SORT);
    let mut models = list_movements(&state.db, &[kind]).await;
    sort_movements(&mut models, &sort);
    let (models, page, total) = slice_page(models, q.page.get(), q.page_size.get());
    let customer_ids: Vec<i64> = models.iter().filter_map(|row| row.customer_id).collect();
    let labels = party::labels(&state.db, &customer_ids).await;
    let driver_ids: Vec<i64> = models.iter().filter_map(|row| row.driver_id).collect();
    let drivers = party::contact_names(&state.db, &driver_ids).await;
    let creator_ids: Vec<i64> = models.iter().filter_map(|row| row.created_by_id).collect();
    let creators = user_names(&state.db, &creator_ids).await;
    let out_ids: Vec<i64> = models
        .iter()
        .filter(|row| row.kind == MovementKind::Out)
        .map(|row| row.id)
        .collect();
    let linked = out_sources_for(&state.db, &out_ids).await;
    let source_in_ids: Vec<i64> = linked.values().flat_map(|row| row.in_ids.clone()).collect();
    let source_process_ids: Vec<i64> = linked
        .values()
        .flat_map(|row| row.process_ids.clone())
        .collect();
    let source_ins = in_movement_labels(&state.db, &source_in_ids).await;
    let source_processes = process_movement_labels(&state.db, &source_process_ids).await;
    let rows = models
        .into_iter()
        .map(|row| {
            let (customer, customer_href) = match row.customer_id.and_then(|id| labels.get(&id)) {
                Some(label) => (label.name.clone(), label.href.clone()),
                None => ("—".into(), String::new()),
            };
            let sources = source_links(linked.get(&row.id), &source_ins, &source_processes);
            MovementRow {
                id: row.id,
                number: row.number,
                customer,
                customer_href,
                datetime: ctx.format_datetime(row.datetime).into_string(),
                vehicle: vehicle_label(row.vehicle_type.as_deref(), row.vehicle_number.as_deref()),
                driver: row
                    .driver_id
                    .and_then(|id| drivers.get(&id).cloned())
                    .unwrap_or_else(|| "—".into()),
                created_by: row
                    .created_by_id
                    .and_then(|id| creators.get(&id).cloned())
                    .unwrap_or_else(|| "—".into()),
                created_at: format_created_at(&ctx, row.created_at),
                sources,
                detail_href: detail_url(row.kind, row.id),
            }
        })
        .collect();
    let page = MovementListPage {
        tab: kind.as_str().to_string(),
        movements: ObjectList::from_page(rows, page, q.page_size.get(), total),
        sort,
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref())
        && MovementTableKey::matches_id(instance)
    {
        return page.render_table_rows(instance);
    }
    if htmx.targets::<MovementTableKey>() {
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

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(kind): Path<String>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(kind) = MovementKind::parse(&kind) else {
        return Redirect::to(&list_url()).into_response();
    };
    let page = empty_create(&ctx, &q, kind);
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn empty_create(
    ctx: &lariv_plugin_users::state::AuthContext,
    q: &ModalNameQuery,
    kind: MovementKind,
) -> MovementCreateModalPage {
    MovementCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        kind: kind.as_str().to_string(),
        number: String::new(),
        datetime: ctx.datetime_local_input(chrono::Utc::now()).into_string(),
        customer_id: String::new(),
        customer_display: String::new(),
        vehicle_type: String::new(),
        vehicle_number: String::new(),
        eway_bill: String::new(),
        driver_id: String::new(),
        driver_display: String::new(),
        source_ins: Vec::new(),
        source_processes: Vec::new(),
        delivery_challan: String::new(),
        delivery_challan_display: String::new(),
        lines_json: crate::logic::line::default_movement_lines_json(),
        input_lines_json: crate::logic::line::default_movement_lines_json(),
        output_lines_json: crate::logic::line::default_movement_lines_json(),
        error: String::new(),
    }
}

pub async fn create_in_post(
    state: Cap<InventoryState>,
    chrome: Cap<SharedChromeFolder>,
    ctx: RequireAuth,
    htmx: Htmx,
    q: Query<ModalNameQuery>,
    form: HtmlFormBody<MovementPartyForm>,
) -> Response {
    create_party(MovementKind::In, state, chrome, ctx, htmx, q, form).await
}

pub async fn create_out_post(
    state: Cap<InventoryState>,
    chrome: Cap<SharedChromeFolder>,
    ctx: RequireAuth,
    htmx: Htmx,
    q: Query<ModalNameQuery>,
    form: HtmlFormBody<MovementPartyForm>,
) -> Response {
    create_party(MovementKind::Out, state, chrome, ctx, htmx, q, form).await
}

async fn create_party(
    kind: MovementKind,
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<MovementPartyForm>,
) -> Response {
    let input = match party_input(&state.db, &ctx, kind, &form).await {
        Ok(input) => input,
        Err(error) => {
            return party_create_error(&state.db, &chrome, &ctx, &q, kind, &form, error).await;
        }
    };
    let saved = match kind {
        MovementKind::In => create_in(&state.db, input)
            .await
            .map(|row| (row.id, row.number)),
        MovementKind::Out => create_out(&state.db, input)
            .await
            .map(|row| (row.id, row.number)),
        MovementKind::Process => Err("process movements use a different form".into()),
    };
    match saved {
        Ok((id, number)) => finish_movement_create(&htmx, &q, kind, id, &number),
        Err(error) => party_create_error(&state.db, &chrome, &ctx, &q, kind, &form, error).await,
    }
}

fn created_picker_label(id: i64, number: &str) -> String {
    let number = number.trim();
    if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

fn finish_movement_create(
    htmx: &Htmx,
    q: &ModalNameQuery,
    kind: MovementKind,
    id: i64,
    number: &str,
) -> Response {
    let detail = detail_url(kind, id);
    let target = q.target_input();
    if target.is_empty() {
        respond_create_modal_done::<MovementCreateModalKey>(htmx, &q.refresh_table(), &detail)
    } else {
        respond_create_modal_done_fk::<MovementCreateModalKey>(
            htmx,
            &q.refresh_table(),
            &detail,
            id,
            &created_picker_label(id, number),
            &target,
        )
    }
}

pub async fn create_process_post(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ProcessMovementForm>,
) -> Response {
    let input = match process_input(&ctx, &form) {
        Ok(input) => input,
        Err(error) => {
            return process_create_error(&chrome, &ctx, &q, &form, error).await;
        }
    };
    match create_process(&state.db, input).await {
        Ok(saved) => {
            finish_movement_create(&htmx, &q, MovementKind::Process, saved.id, &saved.number)
        }
        Err(error) => process_create_error(&chrome, &ctx, &q, &form, error).await,
    }
}

async fn party_create_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    q: &ModalNameQuery,
    kind: MovementKind,
    form: &MovementPartyForm,
    error: String,
) -> Response {
    let displays = party_displays(db, form).await;
    let mut page = empty_create(ctx, q, kind);
    page.number = form.number.clone();
    page.datetime = form.datetime.clone();
    page.customer_id = fk_text(form.customer_id);
    page.customer_display = displays.0;
    page.vehicle_type = form.vehicle_type.clone();
    page.vehicle_number = form.vehicle_number.clone();
    page.eway_bill = form.eway_bill.clone();
    page.driver_id = fk_text(form.driver_id);
    page.driver_display = displays.1;
    let sources = source_items(db, &form.source_in_ids, &form.source_process_ids).await;
    page.source_ins = sources.0;
    page.source_processes = sources.1;
    page.delivery_challan = form.delivery_challan.clone();
    page.delivery_challan_display = displays.2;
    page.lines_json = form.lines_json.clone();
    page.error = error;
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn process_create_error(
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    q: &ModalNameQuery,
    form: &ProcessMovementForm,
    error: String,
) -> Response {
    let mut page = empty_create(ctx, q, MovementKind::Process);
    page.number = form.number.clone();
    page.datetime = form.datetime.clone();
    page.input_lines_json = form.input_lines_json.clone();
    page.output_lines_json = form.output_lines_json.clone();
    page.error = error;
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn detail(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path((kind, id)): Path<(String, i64)>,
) -> Response {
    let Some(kind) = MovementKind::parse(&kind) else {
        return Redirect::to(&list_url()).into_response();
    };
    let page = match kind {
        MovementKind::In => {
            let Some(movement) = find_movement_in(&state.db, id).await else {
                return Redirect::to(&movement_tab_url(kind)).into_response();
            };
            let lines = line_rows(&state.db, &load_in_lines(&state.db, movement.id).await).await;
            party_detail(
                &state.db,
                &ctx,
                kind,
                &movement_party_view(&movement),
                lines,
                Vec::new(),
            )
            .await
        }
        MovementKind::Out => {
            let Some(movement) = find_movement_out(&state.db, id).await else {
                return Redirect::to(&movement_tab_url(kind)).into_response();
            };
            let lines = line_rows(&state.db, &load_out_lines(&state.db, movement.id).await).await;
            party_detail(
                &state.db,
                &ctx,
                kind,
                &movement_party_view_out(&movement),
                lines,
                Vec::new(),
            )
            .await
        }
        MovementKind::Process => {
            let Some(movement) = find_movement_process(&state.db, id).await else {
                return Redirect::to(&movement_tab_url(kind)).into_response();
            };
            let inputs = line_rows(
                &state.db,
                &load_process_lines(&state.db, movement.id, ProcessLineSide::Input).await,
            )
            .await;
            let outputs = line_rows(
                &state.db,
                &load_process_lines(&state.db, movement.id, ProcessLineSide::Output).await,
            )
            .await;
            MovementDetailPage {
                id: movement.id,
                kind: kind.as_str().to_string(),
                number: movement.number,
                datetime: ctx.format_datetime(movement.datetime).into_string(),
                movement_type: kind.label().to_string(),
                show_party: false,
                show_pdf: false,
                show_challan: false,
                customer: String::new(),
                customer_href: String::new(),
                vehicle: String::new(),
                eway_bill: String::new(),
                driver: String::new(),
                driver_href: String::new(),
                delivery_challan: String::new(),
                delivery_challan_href: String::new(),
                created_by: creator_label(&state.db, movement.created_by_id).await,
                created_at: format_created_at(&ctx, movement.created_at),
                show_source: false,
                sources: Vec::new(),
                lines: inputs,
                output_lines: outputs,
            }
        }
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

struct PartyView {
    id: i64,
    number: String,
    datetime: chrono::DateTime<chrono::Utc>,
    customer_id: i64,
    vehicle_type: Option<String>,
    vehicle_number: Option<String>,
    eway_bill: Option<String>,
    driver_id: Option<i64>,
    delivery_challan: Option<i64>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    created_by_id: Option<i64>,
}

fn movement_party_view(row: &crate::entities::stock_movement_in::Model) -> PartyView {
    PartyView {
        id: row.id,
        number: row.number.clone(),
        datetime: row.datetime,
        customer_id: row.customer_id,
        vehicle_type: row.vehicle_type.clone(),
        vehicle_number: row.vehicle_number.clone(),
        eway_bill: row.eway_bill.clone(),
        driver_id: row.driver_id,
        delivery_challan: row.delivery_challan,
        created_at: row.created_at,
        created_by_id: row.created_by_id,
    }
}

fn movement_party_view_out(row: &crate::entities::stock_movement_out::Model) -> PartyView {
    PartyView {
        id: row.id,
        number: row.number.clone(),
        datetime: row.datetime,
        customer_id: row.customer_id,
        vehicle_type: row.vehicle_type.clone(),
        vehicle_number: row.vehicle_number.clone(),
        eway_bill: row.eway_bill.clone(),
        driver_id: row.driver_id,
        delivery_challan: None,
        created_at: row.created_at,
        created_by_id: row.created_by_id,
    }
}

async fn party_detail(
    db: &sea_orm::DatabaseConnection,
    ctx: &lariv_plugin_users::state::AuthContext,
    kind: MovementKind,
    movement: &PartyView,
    lines: Vec<MovementLineRow>,
    output_lines: Vec<MovementLineRow>,
) -> MovementDetailPage {
    let label = party::label(db, movement.customer_id).await;
    let driver = match movement.driver_id {
        Some(id) => party::contact_names(db, &[id])
            .await
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("#{id}")),
        None => "—".into(),
    };
    let (delivery_challan, delivery_challan_href) =
        challan_detail(db, movement.delivery_challan).await;
    let sources = if kind == MovementKind::Out {
        let linked = out_sources_for(db, &[movement.id]).await;
        let sources = linked.get(&movement.id);
        let in_ids = sources.map(|row| row.in_ids.clone()).unwrap_or_default();
        let process_ids = sources
            .map(|row| row.process_ids.clone())
            .unwrap_or_default();
        let source_ins = in_movement_labels(db, &in_ids).await;
        let source_processes = process_movement_labels(db, &process_ids).await;
        source_links(sources, &source_ins, &source_processes)
    } else {
        Vec::new()
    };
    MovementDetailPage {
        id: movement.id,
        kind: kind.as_str().to_string(),
        number: movement.number.clone(),
        datetime: ctx.format_datetime(movement.datetime).into_string(),
        movement_type: kind.label().to_string(),
        show_party: true,
        show_pdf: kind.has_pdf(),
        show_challan: kind == MovementKind::In,
        show_source: kind == MovementKind::Out,
        customer: label.name,
        customer_href: label.href,
        vehicle: vehicle_label(
            movement.vehicle_type.as_deref(),
            movement.vehicle_number.as_deref(),
        ),
        eway_bill: display_or_dash(movement.eway_bill.as_deref()),
        driver,
        driver_href: movement
            .driver_id
            .filter(|id| *id > 0)
            .map(|id| lariv_plugin_contacts::routes::ContactDetailRouteTag::new(id).url())
            .unwrap_or_default(),
        delivery_challan,
        delivery_challan_href,
        sources,
        created_by: creator_label(db, movement.created_by_id).await,
        created_at: format_created_at(ctx, movement.created_at),
        lines,
        output_lines,
    }
}

fn format_created_at(
    ctx: &lariv_plugin_users::state::AuthContext,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    created_at
        .map(|dt| ctx.format_datetime(dt).into_string())
        .unwrap_or_else(|| "—".into())
}

async fn creator_label(db: &sea_orm::DatabaseConnection, created_by_id: Option<i64>) -> String {
    let Some(id) = created_by_id.filter(|id| *id > 0) else {
        return "—".into();
    };
    user_names(db, &[id])
        .await
        .get(&id)
        .cloned()
        .unwrap_or_else(|| format!("#{id}"))
}

async fn line_rows(
    db: &sea_orm::DatabaseConnection,
    lines: &[crate::logic::line::FormLine],
) -> Vec<MovementLineRow> {
    let names = stock_names(
        db,
        &lines.iter().map(|line| line.stock_id).collect::<Vec<_>>(),
    )
    .await;
    lines
        .iter()
        .map(|line| MovementLineRow {
            stock: names.get(&line.stock_id).cloned().unwrap_or_default(),
            qty_display: format_on_hand(line.qty, &line.qty_unit),
            qty_unit: line.qty_unit.clone(),
            qty_type_label: type_label(&line.qty_type),
        })
        .collect()
}

async fn party_input(
    db: &sea_orm::DatabaseConnection,
    ctx: &lariv_plugin_users::state::AuthContext,
    kind: MovementKind,
    form: &MovementPartyForm,
) -> Result<PartyInput, String> {
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        return Err("date and time are required".into());
    };
    let lines = parse_movement_lines_json(&form.lines_json)?;
    let customer_id = party::require_customer(db, form.customer_id).await?;
    let delivery_challan = if kind == MovementKind::In {
        require_challan(db, &form.delivery_challan).await?
    } else {
        None
    };
    let sources = if kind == MovementKind::Out {
        Some(require_out_sources(db, &form.source_in_ids, &form.source_process_ids).await?)
    } else {
        None
    };
    Ok(PartyInput {
        created_by_id: ctx.user.id,
        number: form.number.clone(),
        datetime,
        customer_id,
        vehicle_type: Some(form.vehicle_type.clone()),
        vehicle_number: Some(form.vehicle_number.clone()),
        eway_bill: Some(form.eway_bill.clone()),
        driver_id: Some(form.driver_id).filter(|id| *id > 0),
        delivery_challan,
        sources,
        lines,
    })
}

fn process_input(
    ctx: &lariv_plugin_users::state::AuthContext,
    form: &ProcessMovementForm,
) -> Result<ProcessInput, String> {
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        return Err("date and time are required".into());
    };
    let inputs = parse_movement_lines_json(&form.input_lines_json)
        .map_err(|err| format!("input lines: {err}"))?;
    let outputs = parse_movement_lines_json(&form.output_lines_json)
        .map_err(|err| format!("output lines: {err}"))?;
    Ok(ProcessInput {
        created_by_id: ctx.user.id,
        number: form.number.clone(),
        datetime,
        inputs,
        outputs,
    })
}

fn source_links(
    sources: Option<&crate::logic::movement::OutSources>,
    ins: &std::collections::HashMap<i64, String>,
    processes: &std::collections::HashMap<i64, String>,
) -> Vec<(String, String)> {
    let Some(sources) = sources else {
        return Vec::new();
    };
    let mut links = Vec::new();
    for id in &sources.in_ids {
        let label = ins.get(id).cloned().unwrap_or_else(|| format!("#{id}"));
        links.push((label, detail_url(MovementKind::In, *id)));
    }
    for id in &sources.process_ids {
        let label = processes
            .get(id)
            .cloned()
            .unwrap_or_else(|| format!("#{id}"));
        links.push((label, detail_url(MovementKind::Process, *id)));
    }
    links
}

fn movement_items(
    ids: &[i64],
    labels: &std::collections::HashMap<i64, String>,
) -> Vec<ManyToManyItem> {
    ids.iter()
        .filter(|id| **id > 0)
        .map(|id| {
            ManyToManyItem::new(
                id.to_string(),
                labels.get(id).cloned().unwrap_or_else(|| format!("#{id}")),
            )
        })
        .collect()
}

async fn source_items(
    db: &sea_orm::DatabaseConnection,
    in_ids: &[i64],
    process_ids: &[i64],
) -> (Vec<ManyToManyItem>, Vec<ManyToManyItem>) {
    let ins = in_movement_labels(db, in_ids).await;
    let processes = process_movement_labels(db, process_ids).await;
    (
        movement_items(in_ids, &ins),
        movement_items(process_ids, &processes),
    )
}

fn fk_text(id: i64) -> String {
    if id > 0 {
        id.to_string()
    } else {
        String::new()
    }
}

fn display_or_dash(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "—".into())
}

fn vehicle_label(kind: Option<&str>, number: Option<&str>) -> String {
    let parts: Vec<&str> = [kind, number]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        "—".into()
    } else {
        parts.join(" ")
    }
}

async fn party_displays(
    db: &sea_orm::DatabaseConnection,
    form: &MovementPartyForm,
) -> (String, String, String) {
    let customer = party::label(db, form.customer_id).await.name;
    let driver = if form.driver_id > 0 {
        party::contact_names(db, &[form.driver_id])
            .await
            .get(&form.driver_id)
            .cloned()
            .unwrap_or_default()
    } else {
        String::new()
    };
    let challan = vnode_label(db, &form.delivery_challan).await;
    (customer, driver, challan)
}

async fn vnode_label(db: &sea_orm::DatabaseConnection, raw: &str) -> String {
    let Ok(id) = raw.trim().parse::<i64>() else {
        return String::new();
    };
    if id <= 0 {
        return String::new();
    }
    match VNodeEntity::find_by_id(id).one(db).await {
        Ok(Some(node)) => node.name,
        _ => format!("File #{id}"),
    }
}

async fn challan_detail(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> (String, String) {
    let Some(id) = id.filter(|id| *id > 0) else {
        return ("—".into(), String::new());
    };
    let name = match VNodeEntity::find_by_id(id).one(db).await {
        Ok(Some(node)) => node.name,
        _ => format!("File #{id}"),
    };
    let href = lariv_plugin_filesystem::routes::VNodeDetailRouteTag::new(id).url();
    (name, href)
}

pub async fn edit_get(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path((kind, id)): Path<(String, i64)>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(kind) = MovementKind::parse(&kind) else {
        return Redirect::to(&list_url()).into_response();
    };
    let page = match kind {
        MovementKind::In => {
            let Some(movement) = find_movement_in(&state.db, id).await else {
                return Redirect::to(&movement_tab_url(kind)).into_response();
            };
            let lines = load_in_lines(&state.db, movement.id).await;
            party_edit_page(
                &state.db,
                &ctx,
                &q,
                kind,
                &movement_party_view(&movement),
                &lines,
            )
            .await
        }
        MovementKind::Out => {
            let Some(movement) = find_movement_out(&state.db, id).await else {
                return Redirect::to(&movement_tab_url(kind)).into_response();
            };
            let lines = load_out_lines(&state.db, movement.id).await;
            party_edit_page(
                &state.db,
                &ctx,
                &q,
                kind,
                &movement_party_view_out(&movement),
                &lines,
            )
            .await
        }
        MovementKind::Process => {
            let Some(movement) = find_movement_process(&state.db, id).await else {
                return Redirect::to(&movement_tab_url(kind)).into_response();
            };
            let inputs = load_process_lines(&state.db, movement.id, ProcessLineSide::Input).await;
            let outputs = load_process_lines(&state.db, movement.id, ProcessLineSide::Output).await;
            MovementEditModalPage {
                id: movement.id,
                form_name: q.form_name(),
                kind: kind.as_str().to_string(),
                number: movement.number,
                datetime: ctx.datetime_local_input(movement.datetime).into_string(),
                customer_id: String::new(),
                customer_display: String::new(),
                vehicle_type: String::new(),
                vehicle_number: String::new(),
                eway_bill: String::new(),
                driver_id: String::new(),
                driver_display: String::new(),
                source_ins: Vec::new(),
                source_processes: Vec::new(),
                delivery_challan: String::new(),
                delivery_challan_display: String::new(),
                lines_json: String::new(),
                input_lines_json: lines_form_json(&state.db, &inputs, "process-in-").await,
                output_lines_json: lines_form_json(&state.db, &outputs, "process-out-").await,
                error: String::new(),
            }
        }
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn party_edit_page(
    db: &sea_orm::DatabaseConnection,
    ctx: &lariv_plugin_users::state::AuthContext,
    q: &ModalNameQuery,
    kind: MovementKind,
    movement: &PartyView,
    lines: &[crate::logic::line::FormLine],
) -> MovementEditModalPage {
    let customer = party::label(db, movement.customer_id).await;
    let driver_display = match movement.driver_id {
        Some(id) if id > 0 => party::contact_names(db, &[id])
            .await
            .get(&id)
            .cloned()
            .unwrap_or_default(),
        _ => String::new(),
    };
    let challan_raw = id_value(movement.delivery_challan);
    let delivery_challan_display = vnode_label(db, &challan_raw).await;
    let linked = out_sources_for(db, &[movement.id]).await;
    let linked = linked.get(&movement.id).cloned().unwrap_or_default();
    let sources = source_items(db, &linked.in_ids, &linked.process_ids).await;
    MovementEditModalPage {
        id: movement.id,
        form_name: q.form_name(),
        kind: kind.as_str().to_string(),
        number: movement.number.clone(),
        datetime: ctx.datetime_local_input(movement.datetime).into_string(),
        customer_id: fk_text(movement.customer_id),
        customer_display: customer.name,
        vehicle_type: movement.vehicle_type.clone().unwrap_or_default(),
        vehicle_number: movement.vehicle_number.clone().unwrap_or_default(),
        eway_bill: movement.eway_bill.clone().unwrap_or_default(),
        driver_id: fk_text(movement.driver_id.unwrap_or(0)),
        driver_display,
        source_ins: sources.0,
        source_processes: sources.1,
        delivery_challan: challan_raw,
        delivery_challan_display,
        lines_json: lines_form_json(db, lines, &format!("{}-", kind.as_str())).await,
        input_lines_json: String::new(),
        output_lines_json: String::new(),
        error: String::new(),
    }
}

pub async fn edit_in_post(
    state: Cap<InventoryState>,
    chrome: Cap<SharedChromeFolder>,
    ctx: RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    q: Query<ModalNameQuery>,
    form: HtmlFormBody<MovementPartyForm>,
) -> Response {
    edit_party(MovementKind::In, state, chrome, ctx, htmx, id, q, form).await
}

pub async fn edit_out_post(
    state: Cap<InventoryState>,
    chrome: Cap<SharedChromeFolder>,
    ctx: RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    q: Query<ModalNameQuery>,
    form: HtmlFormBody<MovementPartyForm>,
) -> Response {
    edit_party(MovementKind::Out, state, chrome, ctx, htmx, id, q, form).await
}

#[expect(
    clippy::too_many_arguments,
    reason = "kind, request extractors, id, and the submitted form stay explicit"
)]
async fn edit_party(
    kind: MovementKind,
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    id: i64,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<MovementPartyForm>,
) -> Response {
    let input = match party_input(&state.db, &ctx, kind, &form).await {
        Ok(input) => input,
        Err(error) => {
            return party_edit_error(&state.db, &chrome, &ctx, id, kind, &q, &form, error).await;
        }
    };
    let saved = match kind {
        MovementKind::In => update_in(&state.db, id, input).await.map(|_| ()),
        MovementKind::Out => update_out(&state.db, id, input).await.map(|_| ()),
        MovementKind::Process => Err("process movements use a different form".into()),
    };
    match saved {
        Ok(()) => respond_edit_modal_done::<MovementEditModalKey>(&htmx, &detail_url(kind, id)),
        Err(error) => party_edit_error(&state.db, &chrome, &ctx, id, kind, &q, &form, error).await,
    }
}

pub async fn edit_process_post(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ProcessMovementForm>,
) -> Response {
    let input = match process_input(&ctx, &form) {
        Ok(input) => input,
        Err(error) => {
            return process_edit_error(&chrome, &ctx, id, &q, &form, error).await;
        }
    };
    match update_process(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<MovementEditModalKey>(
            &htmx,
            &detail_url(MovementKind::Process, id),
        ),
        Err(error) => process_edit_error(&chrome, &ctx, id, &q, &form, error).await,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "error page needs the connection, chrome, auth, id, kind, query, and form"
)]
async fn party_edit_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    id: i64,
    kind: MovementKind,
    q: &ModalNameQuery,
    form: &MovementPartyForm,
    error: String,
) -> Response {
    let displays = party_displays(db, form).await;
    let sources = source_items(db, &form.source_in_ids, &form.source_process_ids).await;
    let page = MovementEditModalPage {
        id,
        form_name: q.form_name(),
        kind: kind.as_str().to_string(),
        number: form.number.clone(),
        datetime: form.datetime.clone(),
        customer_id: fk_text(form.customer_id),
        customer_display: displays.0,
        vehicle_type: form.vehicle_type.clone(),
        vehicle_number: form.vehicle_number.clone(),
        eway_bill: form.eway_bill.clone(),
        driver_id: fk_text(form.driver_id),
        driver_display: displays.1,
        source_ins: sources.0,
        source_processes: sources.1,
        delivery_challan: form.delivery_challan.clone(),
        delivery_challan_display: displays.2,
        lines_json: form.lines_json.clone(),
        input_lines_json: String::new(),
        output_lines_json: String::new(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn process_edit_error(
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &ProcessMovementForm,
    error: String,
) -> Response {
    let page = MovementEditModalPage {
        id,
        form_name: q.form_name(),
        kind: MovementKind::Process.as_str().to_string(),
        number: form.number.clone(),
        datetime: form.datetime.clone(),
        customer_id: String::new(),
        customer_display: String::new(),
        vehicle_type: String::new(),
        vehicle_number: String::new(),
        eway_bill: String::new(),
        driver_id: String::new(),
        driver_display: String::new(),
        source_ins: Vec::new(),
        source_processes: Vec::new(),
        delivery_challan: String::new(),
        delivery_challan_display: String::new(),
        lines_json: String::new(),
        input_lines_json: form.input_lines_json.clone(),
        output_lines_json: form.output_lines_json.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path((kind, id)): Path<(String, i64)>,
) -> Response {
    let Some(kind) = MovementKind::parse(&kind) else {
        return Redirect::to(&list_url()).into_response();
    };
    let page = delete_page(&q, kind, id, String::new());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn delete_page(
    q: &ModalNameQuery,
    kind: MovementKind,
    id: i64,
    error: String,
) -> ConfirmDeletePage {
    ConfirmDeletePage {
        modal_uid: MovementDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this movement? Its lines will be deleted too."
            .into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_inventory.MovementDeleteForm".into()),
        id,
        post_url: crate::routes::MovementDeletePostRouteTag::new(kind.as_str().to_string(), id)
            .url(),
        error,
    }
}

pub async fn delete_post(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path((kind, id)): Path<(String, i64)>,
) -> Response {
    let Some(kind) = MovementKind::parse(&kind) else {
        return Redirect::to(&list_url()).into_response();
    };
    let deleted = match kind {
        MovementKind::In => delete_in(&state.db, id).await,
        MovementKind::Out => delete_out(&state.db, id).await,
        MovementKind::Process => delete_process(&state.db, id).await,
    };
    match deleted {
        Ok(()) => htmx.redirect(&movement_tab_url(kind)),
        Err(error) => {
            tracing::error!(error = %error, id, "failed to delete movement");
            let page = ConfirmDeletePage {
                modal_uid: MovementDeleteModalKey::ID.to_string(),
                message:
                    "Are you sure you want to delete this movement? Its lines will be deleted too."
                        .into(),
                form_name: "p_inventory.MovementDeleteForm".into(),
                id,
                post_url: crate::routes::MovementDeletePostRouteTag::new(
                    kind.as_str().to_string(),
                    id,
                )
                .url(),
                error,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct SourceSelectQuery {
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
    #[serde(default, rename = "Name")]
    name: Option<String>,
    #[serde(default)]
    target_input: Option<String>,
}

pub async fn select_in(
    Cap(state): Cap<InventoryState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<SourceSelectQuery>,
) -> maud::Markup {
    let mut query = scope_allowed::<InventoryView, _>(InEntity::find());
    if let Some(name) = q
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        query = query.filter(stock_movement_in::Column::Number.contains(name));
    }
    query = query.order_by_desc(stock_movement_in::Column::Datetime);
    let (models, page, total) = page_of(query, &state.db, q.page.get(), q.page_size.get()).await;
    let rows = models
        .into_iter()
        .map(|row| {
            let when = ctx.format_datetime(row.datetime).into_string();
            let label = if row.number.trim().is_empty() {
                when.clone()
            } else {
                row.number
            };
            (row.id, label, when)
        })
        .collect();
    let page = MovementSourceSelectPage {
        title: "Select in movement".into(),
        rows: ObjectList::from_page(rows, page, q.page_size.get(), total),
        target_input: q.target_input.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<MovementInSelectTableKey, MovementInSelectModalKey, _>(&htmx, &page)
}

pub async fn select_process(
    Cap(state): Cap<InventoryState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<SourceSelectQuery>,
) -> maud::Markup {
    let mut query = scope_allowed::<InventoryView, _>(ProcessEntity::find());
    if let Some(name) = q
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        query = query.filter(stock_movement_process::Column::Number.contains(name));
    }
    query = query.order_by_desc(stock_movement_process::Column::Datetime);
    let (models, page, total) = page_of(query, &state.db, q.page.get(), q.page_size.get()).await;
    let rows = models
        .into_iter()
        .map(|row| {
            let when = ctx.format_datetime(row.datetime).into_string();
            let label = if row.number.trim().is_empty() {
                when.clone()
            } else {
                row.number
            };
            (row.id, label, when)
        })
        .collect();
    let page = MovementSourceSelectPage {
        title: "Select process".into(),
        rows: ObjectList::from_page(rows, page, q.page_size.get(), total),
        target_input: q.target_input.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<MovementProcessSelectTableKey, MovementProcessSelectModalKey, _>(
        &htmx, &page,
    )
}
