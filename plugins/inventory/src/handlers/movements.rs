use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::EntityTrait;

use lariv_core::components::{
    ObjectList, SharedChromeFolder, SlotCtx, SwapKey, table_rows_instance_id,
};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::role_authorization::scope_allowed;

use crate::{
    entities::stock_movement::{self, Entity as MovementEntity},
    forms::MovementForm,
    handlers::{ModalNameQuery, path_and_query},
    keys::{
        MovementCreateModalKey, MovementDeleteModalKey, MovementEditModalKey, MovementTableKey,
    },
    logic::{
        line::{movement_lines_form_json, parse_movement_lines_json},
        movement::{MovementInput, create_movement, delete_movement, update_movement},
        party::{self, BillTo},
        qty::{format_on_hand, type_label},
    },
    movement_type::MovementType,
    routes::{InventoryView, MovementDefaultRouteTag, MovementDetailRouteTag},
    scope::{
        DEFAULT_MOVEMENT_SORT, apply_movement_sort, effective_sort, find_movement, page_of,
        stock_names,
    },
    state::InventoryState,
    templates::{
        ConfirmDeletePage, MovementCreateModalPage, MovementDetailPage, MovementEditModalPage,
        MovementLineRow, MovementListPage, MovementRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct MovementListQuery {
    #[serde(default, rename = "MovementType", alias = "movement_type")]
    pub movement_type: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn list_url() -> String {
    MovementDefaultRouteTag.url()
}

pub async fn list(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<MovementListQuery>,
) -> maud::Markup {
    use sea_orm::{ColumnTrait, QueryFilter};

    let sort = effective_sort(q.sort.as_deref(), DEFAULT_MOVEMENT_SORT);
    let mut query = scope_allowed::<InventoryView, _>(MovementEntity::find());
    if let Some(raw) = q.movement_type.as_deref().filter(|s| !s.is_empty())
        && let Some(kind) = MovementType::parse(raw)
    {
        query = query.filter(stock_movement::Column::MovementType.eq(kind));
    }
    query = apply_movement_sort(query, &sort);
    let (models, page, total) = page_of(query, &state.db, q.page.get(), q.page_size.get()).await;
    let parties: Vec<BillTo> = models
        .iter()
        .map(|row| {
            BillTo::from_row(
                row.bill_to_individual,
                row.customer_individual,
                row.customer_company,
            )
        })
        .collect();
    let labels = party::PartyLabels::load(&state.db, &parties).await;
    let driver_ids: Vec<i64> = models.iter().filter_map(|row| row.driver_id).collect();
    let drivers = party::contact_names(&state.db, &driver_ids).await;
    let rows = models
        .into_iter()
        .map(|row| {
            let bill = BillTo::from_row(
                row.bill_to_individual,
                row.customer_individual,
                row.customer_company,
            );
            MovementRow {
                id: row.id,
                number: row.number,
                customer: labels.name(bill),
                customer_href: customer_href(bill),
                datetime: ctx.format_datetime(row.datetime).into_string(),
                movement_type: row.movement_type.label().to_string(),
                vehicle: vehicle_label(row.vehicle_type.as_deref(), row.vehicle_number.as_deref()),
                driver: row
                    .driver_id
                    .and_then(|id| drivers.get(&id).cloned())
                    .unwrap_or_else(|| "—".into()),
                detail_href: MovementDetailRouteTag::new(row.id).url(),
            }
        })
        .collect();
    let page = MovementListPage {
        movements: ObjectList::from_page(rows, page, q.page_size.get(), total),
        filter_type: q.movement_type.clone().unwrap_or_default(),
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
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = MovementCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        number: String::new(),
        datetime: ctx.datetime_local_input(chrono::Utc::now()).into_string(),
        movement_type: MovementType::In.as_str().to_string(),
        bill_to_individual: false,
        customer_individual: String::new(),
        customer_individual_display: String::new(),
        customer_company: String::new(),
        customer_company_display: String::new(),
        vehicle_type: String::new(),
        vehicle_number: String::new(),
        eway_bill: String::new(),
        driver_id: String::new(),
        driver_display: String::new(),
        lines_json: crate::logic::line::default_movement_lines_json(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<MovementForm>,
) -> Response {
    let input = match movement_input(&ctx, &form) {
        Ok(input) => input,
        Err(error) => {
            return create_error(&state.db, &chrome, &ctx, &q, &form, error).await;
        }
    };
    match create_movement(&state.db, input).await {
        Ok(saved) => respond_create_modal_done::<MovementCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &MovementDetailRouteTag::new(saved.id).url(),
        ),
        Err(error) => create_error(&state.db, &chrome, &ctx, &q, &form, error).await,
    }
}

async fn create_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    q: &ModalNameQuery,
    form: &MovementForm,
    error: String,
) -> Response {
    let displays = form_displays(db, form).await;
    let page = MovementCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        number: form.number.clone(),
        datetime: form.datetime.clone(),
        movement_type: form.movement_type.clone(),
        bill_to_individual: party::checkbox_on(&form.bill_to_individual),
        customer_individual: fk_text(form.customer_individual),
        customer_individual_display: displays.0,
        customer_company: fk_text(form.customer_company),
        customer_company_display: displays.1,
        vehicle_type: form.vehicle_type.clone(),
        vehicle_number: form.vehicle_number.clone(),
        eway_bill: form.eway_bill.clone(),
        driver_id: fk_text(form.driver_id),
        driver_display: displays.2,
        lines_json: form.lines_json.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn detail(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(movement) = find_movement(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let lines = movement_line_rows(&state.db, movement.id).await;
    let bill = BillTo::from_row(
        movement.bill_to_individual,
        movement.customer_individual,
        movement.customer_company,
    );
    let labels = party::PartyLabels::load(&state.db, &[bill]).await;
    let driver = match movement.driver_id {
        Some(id) => party::contact_names(&state.db, &[id])
            .await
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("#{id}")),
        None => "—".into(),
    };
    let datetime = ctx.format_datetime(movement.datetime).into_string();
    let page = MovementDetailPage {
        id: movement.id,
        number: movement.number.clone(),
        datetime,
        movement_type: movement.movement_type.label().to_string(),
        customer: labels.name(bill),
        customer_href: customer_href(bill),
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
        lines,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn movement_line_rows(
    db: &sea_orm::DatabaseConnection,
    movement_id: i64,
) -> Vec<MovementLineRow> {
    use sea_orm::{ColumnTrait, QueryFilter};

    use crate::entities::stock_movement_line::{self, Entity as LineEntity};

    let models = scope_allowed::<InventoryView, _>(LineEntity::find())
        .filter(stock_movement_line::Column::StockMovementId.eq(movement_id))
        .all(db)
        .await
        .unwrap_or_default();
    let names = stock_names(
        db,
        &models.iter().map(|line| line.stock_id).collect::<Vec<_>>(),
    )
    .await;
    models
        .into_iter()
        .map(|line| MovementLineRow {
            stock: names.get(&line.stock_id).cloned().unwrap_or_default(),
            qty_display: format_on_hand(line.qty, &line.qty_unit),
            qty_unit: line.qty_unit,
            qty_type_label: type_label(&line.qty_type),
        })
        .collect()
}

fn movement_input(
    ctx: &lariv_plugin_users::state::AuthContext,
    form: &MovementForm,
) -> Result<MovementInput, String> {
    let Some(datetime) = ctx.parse_datetime_local_input(&form.datetime) else {
        return Err("date and time are required".into());
    };
    let Some(movement_type) = MovementType::parse(&form.movement_type) else {
        return Err("type is required".into());
    };
    let lines = parse_movement_lines_json(&form.lines_json)?;
    let bill_to = party::require_bill_to(
        party::checkbox_on(&form.bill_to_individual),
        form.customer_individual,
        form.customer_company,
    )?;
    Ok(MovementInput {
        number: form.number.clone(),
        datetime,
        movement_type,
        bill_to,
        vehicle_type: Some(form.vehicle_type.clone()),
        vehicle_number: Some(form.vehicle_number.clone()),
        eway_bill: Some(form.eway_bill.clone()),
        driver_id: Some(form.driver_id).filter(|id| *id > 0),
        lines,
    })
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

fn customer_href(bill: BillTo) -> String {
    let id = bill.party_id();
    if id <= 0 {
        return String::new();
    }
    if bill.bill_to_individual {
        lariv_plugin_contacts::routes::ContactDetailRouteTag::new(id).url()
    } else {
        lariv_plugin_contacts::routes::CompanyDetailRouteTag::new(id).url()
    }
}

async fn form_displays(
    db: &sea_orm::DatabaseConnection,
    form: &MovementForm,
) -> (String, String, String) {
    let (individual, company) =
        party::party_displays(db, form.customer_individual, form.customer_company).await;
    let driver = if form.driver_id > 0 {
        party::contact_names(db, &[form.driver_id])
            .await
            .get(&form.driver_id)
            .cloned()
            .unwrap_or_default()
    } else {
        String::new()
    };
    (individual, company, driver)
}

pub async fn edit_get(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(movement) = find_movement(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let lines_json = movement_lines_form_json(&state.db, movement.id).await;
    let (customer_individual_display, customer_company_display) = party::party_displays(
        &state.db,
        movement.customer_individual.unwrap_or(0),
        movement.customer_company.unwrap_or(0),
    )
    .await;
    let driver_display = match movement.driver_id {
        Some(id) if id > 0 => party::contact_names(&state.db, &[id])
            .await
            .get(&id)
            .cloned()
            .unwrap_or_default(),
        _ => String::new(),
    };
    let page = MovementEditModalPage {
        id: movement.id,
        form_name: q.form_name(),
        number: movement.number,
        datetime: ctx.datetime_local_input(movement.datetime).into_string(),
        movement_type: movement.movement_type.as_str().to_string(),
        bill_to_individual: movement.bill_to_individual,
        customer_individual: fk_text(movement.customer_individual.unwrap_or(0)),
        customer_individual_display,
        customer_company: fk_text(movement.customer_company.unwrap_or(0)),
        customer_company_display,
        vehicle_type: movement.vehicle_type.unwrap_or_default(),
        vehicle_number: movement.vehicle_number.unwrap_or_default(),
        eway_bill: movement.eway_bill.unwrap_or_default(),
        driver_id: fk_text(movement.driver_id.unwrap_or(0)),
        driver_display,
        lines_json,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<MovementForm>,
) -> Response {
    let input = match movement_input(&ctx, &form) {
        Ok(input) => input,
        Err(error) => {
            return edit_error(&state.db, &chrome, &ctx, id, &q, &form, error).await;
        }
    };
    match update_movement(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<MovementEditModalKey>(
            &htmx,
            &MovementDetailRouteTag::new(id).url(),
        ),
        Err(error) => edit_error(&state.db, &chrome, &ctx, id, &q, &form, error).await,
    }
}

async fn edit_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &MovementForm,
    error: String,
) -> Response {
    let displays = form_displays(db, form).await;
    let page = MovementEditModalPage {
        id,
        form_name: q.form_name(),
        number: form.number.clone(),
        datetime: form.datetime.clone(),
        movement_type: form.movement_type.clone(),
        bill_to_individual: party::checkbox_on(&form.bill_to_individual),
        customer_individual: fk_text(form.customer_individual),
        customer_individual_display: displays.0,
        customer_company: fk_text(form.customer_company),
        customer_company_display: displays.1,
        vehicle_type: form.vehicle_type.clone(),
        vehicle_number: form.vehicle_number.clone(),
        eway_bill: form.eway_bill.clone(),
        driver_id: fk_text(form.driver_id),
        driver_display: displays.2,
        lines_json: form.lines_json.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: MovementDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this movement? Its lines will be deleted too."
            .into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_inventory.MovementDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match delete_movement(&state.db, id).await {
        Ok(()) => htmx.redirect(&list_url()),
        Err(error) => {
            tracing::error!(error = %error, id, "failed to delete movement");
            let page = ConfirmDeletePage {
                modal_uid: MovementDeleteModalKey::ID.to_string(),
                message:
                    "Are you sure you want to delete this movement? Its lines will be deleted too."
                        .into(),
                form_name: "p_inventory.MovementDeleteForm".into(),
                id,
                error,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
