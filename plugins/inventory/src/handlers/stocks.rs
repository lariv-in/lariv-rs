use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use lariv_core::components::{
    ObjectList, SharedChromeFolder, SlotCtx, SwapKey, table_rows_instance_id,
};
use lariv_core::db::trigram::ci_contains;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::picker::respond_picker_select;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done_fk_extra, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::role_authorization::scope_allowed;

use crate::{
    entities::stock::{self, Entity as StockEntity},
    forms::StockForm,
    handlers::{ModalNameQuery, path_and_query},
    keys::{
        StockCreateModalKey, StockDeleteModalKey, StockEditModalKey, StockLinesTableKey,
        StockSelectModalKey, StockSelectTableKey, StockTableKey,
    },
    logic::{
        qty::{format_on_hand, type_label},
        stock::{StockInput, create_stock, delete_stock, update_stock},
    },
    routes::{InventoryView, StockDefaultRouteTag, StockDetailRouteTag},
    scope::{
        DEFAULT_STOCK_SORT, apply_stock_sort, company_name, company_names, effective_sort,
        find_stock, on_hand, on_hand_for_stocks, page_of,
    },
    state::InventoryState,
    templates::{
        ConfirmDeletePage, StockCreateModalPage, StockDetailPage, StockEditModalPage, StockLineRow,
        StockListPage, StockRow, StockSelectPage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct StockListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct StockSelectQuery {
    #[serde(flatten)]
    pub filter: StockListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct StockDetailQuery {
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn list_url() -> String {
    StockDefaultRouteTag.url()
}

fn stock_input(form: &StockForm) -> StockInput {
    StockInput {
        name: form.name.clone(),
        company_id: form.company_id,
        qty_type: form.qty_type.clone(),
        qty_unit: form.qty_unit.clone(),
    }
}

pub async fn list(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<StockListQuery>,
) -> maud::Markup {
    let sort = effective_sort(q.sort.as_deref(), DEFAULT_STOCK_SORT);
    let mut query = scope_allowed::<InventoryView, _>(StockEntity::find());
    if let Some(name) = q.name.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(ci_contains(stock::Column::Name, name));
    }
    query = apply_stock_sort(query, &sort);
    let (models, page, total) = page_of(query, &state.db, q.page.get(), q.page_size.get()).await;
    let names = company_names(
        &state.db,
        &models.iter().map(|s| s.company_id).collect::<Vec<_>>(),
    )
    .await;
    let qty = on_hand_for_stocks(&state.db, &models).await;
    let rows = models
        .iter()
        .map(|s| {
            let on_hand = match qty.get(&s.id) {
                Some(Ok(value)) => format_on_hand(*value, &s.qty_unit),
                Some(Err(err)) => err.clone(),
                None => format_on_hand(rust_decimal::Decimal::ZERO, &s.qty_unit),
            };
            StockRow {
                id: s.id,
                name: s.name.clone(),
                company: names.get(&s.company_id).cloned().unwrap_or_default(),
                qty_type: type_label(&s.qty_type),
                qty_unit: s.qty_unit.clone(),
                qty: on_hand,
                detail_href: StockDetailRouteTag::new(s.id).url(),
            }
        })
        .collect();
    let page = StockListPage {
        stocks: ObjectList::from_page(rows, page, q.page_size.get(), total),
        filter_name: q.name.clone().unwrap_or_default(),
        sort,
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref())
        && StockTableKey::matches_id(instance)
    {
        return page.render_table_rows(instance);
    }
    if htmx.targets::<StockTableKey>() {
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

pub async fn select(
    Cap(state): Cap<InventoryState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<StockSelectQuery>,
) -> maud::Markup {
    let sort = effective_sort(q.filter.sort.as_deref(), DEFAULT_STOCK_SORT);
    let mut query = scope_allowed::<InventoryView, _>(StockEntity::find());
    if let Some(name) = q.filter.name.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(ci_contains(stock::Column::Name, name));
    }
    query = apply_stock_sort(query, &sort);
    let (models, page, total) = page_of(
        query,
        &state.db,
        q.filter.page.get(),
        q.filter.page_size.get(),
    )
    .await;
    let rows = models
        .into_iter()
        .map(|s| StockRow {
            id: s.id,
            name: s.name,
            company: String::new(),
            qty_type: s.qty_type,
            qty_unit: s.qty_unit,
            qty: String::new(),
            detail_href: String::new(),
        })
        .collect();
    let page = StockSelectPage {
        stocks: ObjectList::from_page(rows, page, q.filter.page_size.get(), total),
        filter_name: q.filter.name.clone().unwrap_or_default(),
        sort,
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.unwrap_or_else(|| "StockID".into()),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select::<StockSelectTableKey, StockSelectModalKey, _>(&htmx, &page)
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = StockCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        company_id: 0,
        company_display: String::new(),
        qty_type: "quantity".into(),
        qty_unit: String::new(),
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
    HtmlFormBody(form): HtmlFormBody<StockForm>,
) -> Response {
    match create_stock(&state.db, stock_input(&form)).await {
        Ok(saved) => respond_create_modal_done_fk_extra::<StockCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &StockDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
            &[
                ("qty_type", saved.qty_type.as_str()),
                ("qty_unit", saved.qty_unit.as_str()),
            ],
        ),
        Err(error) => {
            let company_display = company_name(&state.db, form.company_id).await;
            let page = StockCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                company_id: form.company_id,
                company_display,
                qty_type: form.qty_type,
                qty_unit: form.qty_unit,
                error,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Path(id): Path<i64>,
    Query(q): Query<StockDetailQuery>,
) -> Response {
    let Some(stock) = find_stock(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let qty = on_hand(&state.db, &stock)
        .await
        .map(|value| format_on_hand(value, &stock.qty_unit))
        .unwrap_or_else(|err| err);
    let company = company_name(&state.db, stock.company_id).await;
    let lines = stock_lines(&state.db, stock.id, q.page.get(), q.page_size.get(), &ctx).await;
    let page = StockDetailPage {
        id: stock.id,
        name: stock.name,
        company,
        qty_type: type_label(&stock.qty_type),
        qty_unit: stock.qty_unit,
        qty,
        lines: ObjectList::from_page(lines.0, lines.1, q.page_size.get(), lines.2),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    if let Some(instance) = table_rows_instance_id(htmx.target_id.as_deref())
        && StockLinesTableKey::matches_id(instance)
    {
        return page.render_lines_rows(instance).into_response();
    }
    if htmx.targets::<StockLinesTableKey>() {
        return page.render_lines().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn stock_lines(
    db: &sea_orm::DatabaseConnection,
    stock_id: i64,
    page: u32,
    page_size: u32,
    ctx: &lariv_plugin_users::state::AuthContext,
) -> (Vec<StockLineRow>, u32, u64) {
    use crate::entities::stock_movement::{self, Entity as MovementEntity};
    use crate::entities::stock_movement_line::{self, Entity as LineEntity};
    use crate::routes::MovementDetailRouteTag;

    let query = scope_allowed::<InventoryView, _>(LineEntity::find())
        .filter(stock_movement_line::Column::StockId.eq(stock_id));
    let (models, page, total) = page_of(query, db, page, page_size).await;
    let movement_ids: Vec<i64> = models.iter().map(|line| line.stock_movement_id).collect();
    let movements = if movement_ids.is_empty() {
        Vec::new()
    } else {
        MovementEntity::find()
            .filter(stock_movement::Column::Id.is_in(movement_ids))
            .all(db)
            .await
            .unwrap_or_default()
    };
    let by_id: std::collections::HashMap<i64, stock_movement::Model> =
        movements.into_iter().map(|row| (row.id, row)).collect();
    let rows = models
        .into_iter()
        .map(|line| {
            let movement = by_id.get(&line.stock_movement_id);
            let when = movement
                .map(|row| ctx.format_datetime(row.datetime).into_string())
                .unwrap_or_default();
            let kind = movement
                .map(|row| row.movement_type.label().to_string())
                .unwrap_or_default();
            StockLineRow {
                when,
                movement_type: kind,
                qty: format_on_hand(line.qty, &line.qty_unit),
                qty_type: type_label(&line.qty_type),
                detail_href: MovementDetailRouteTag::new(line.stock_movement_id).url(),
            }
        })
        .collect();
    (rows, page, total)
}

pub async fn edit_get(
    Cap(state): Cap<InventoryState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(stock) = find_stock(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let page = StockEditModalPage {
        id: stock.id,
        form_name: q.form_name(),
        name: stock.name,
        company_id: stock.company_id,
        company_display: company_name(&state.db, stock.company_id).await,
        qty_type: stock.qty_type,
        qty_unit: stock.qty_unit,
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
    HtmlFormBody(form): HtmlFormBody<StockForm>,
) -> Response {
    match update_stock(&state.db, id, stock_input(&form)).await {
        Ok(_) => {
            respond_edit_modal_done::<StockEditModalKey>(&htmx, &StockDetailRouteTag::new(id).url())
        }
        Err(error) => {
            let page = StockEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                company_id: form.company_id,
                company_display: company_name(&state.db, form.company_id).await,
                qty_type: form.qty_type,
                qty_unit: form.qty_unit,
                error,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: StockDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this stock? It cannot be deleted while movement lines reference it.".into(),
        form_name: q.name.clone().unwrap_or_else(|| "p_inventory.StockDeleteForm".into()),
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
    match delete_stock(&state.db, id).await {
        Ok(()) => htmx.redirect(&list_url()),
        Err(error) => {
            tracing::error!(error = %error, id, "failed to delete stock");
            let page = ConfirmDeletePage {
                modal_uid: StockDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this stock? It cannot be deleted while movement lines reference it.".into(),
                form_name: "p_inventory.StockDeleteForm".into(),
                id,
                error,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
