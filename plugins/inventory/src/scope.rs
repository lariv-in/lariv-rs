use std::collections::HashMap;

use lariv_plugin_contacts::entities::company::{self, Entity as CompanyEntity};
use lariv_plugin_users::entities::user::{self, Entity as UserEntity};
use lariv_plugin_users::role_authorization::scope_allowed;
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Select,
};

use crate::entities::{
    stock::{self, Entity as StockEntity},
    stock_movement_in::{self, Entity as InEntity},
    stock_movement_in_line::{self, Entity as InLineEntity},
    stock_movement_out::{self, Entity as OutEntity},
    stock_movement_out_line::{self, Entity as OutLineEntity},
    stock_movement_process::{self, Entity as ProcessEntity},
    stock_movement_process_line::{self, Entity as ProcessLineEntity, ProcessLineSide},
};
use crate::logic::party::BillTo;
use crate::logic::qty::{QtyLine, QtySign, sum_on_hand};
use crate::movement_type::MovementKind;

pub const DEFAULT_STOCK_SORT: &str = "Name ASC";
pub const DEFAULT_MOVEMENT_SORT: &str = "Datetime DESC";

pub fn effective_sort(sort: Option<&str>, default: &str) -> String {
    sort.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default.to_string())
}

pub async fn find_stock(db: &DatabaseConnection, id: i64) -> Option<stock::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::InventoryView, _>(StockEntity::find_by_id(id))
            .one(db)
            .await,
        "find stock",
    )
}

pub async fn find_movement_in(
    db: &DatabaseConnection,
    id: i64,
) -> Option<stock_movement_in::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::InventoryView, _>(InEntity::find_by_id(id))
            .one(db)
            .await,
        "find movement in",
    )
}

pub async fn find_movement_out(
    db: &DatabaseConnection,
    id: i64,
) -> Option<stock_movement_out::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::InventoryView, _>(OutEntity::find_by_id(id))
            .one(db)
            .await,
        "find movement out",
    )
}

pub async fn find_movement_process(
    db: &DatabaseConnection,
    id: i64,
) -> Option<stock_movement_process::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::InventoryView, _>(ProcessEntity::find_by_id(id))
            .one(db)
            .await,
        "find movement process",
    )
}

pub async fn user_names(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    UserEntity::find()
        .filter(user::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect()
}

/// One row in the combined movements list.
pub struct ListedMovement {
    pub id: i64,
    pub kind: MovementKind,
    pub number: String,
    pub datetime: chrono::DateTime<chrono::Utc>,
    pub bill_to: Option<BillTo>,
    pub vehicle_type: Option<String>,
    pub vehicle_number: Option<String>,
    pub driver_id: Option<i64>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by_id: Option<i64>,
}

pub async fn list_movements(
    db: &DatabaseConnection,
    kinds: &[MovementKind],
) -> Vec<ListedMovement> {
    let mut rows = Vec::new();
    if kinds.contains(&MovementKind::In) {
        let models = scope_allowed::<super::routes::InventoryView, _>(InEntity::find())
            .all(db)
            .await
            .unwrap_or_default();
        rows.extend(models.into_iter().map(|row| ListedMovement {
            id: row.id,
            kind: MovementKind::In,
            number: row.number,
            datetime: row.datetime,
            bill_to: Some(BillTo::from_row(
                row.bill_to_individual,
                row.customer_individual,
                row.customer_company,
            )),
            vehicle_type: row.vehicle_type,
            vehicle_number: row.vehicle_number,
            driver_id: row.driver_id,
            created_at: row.created_at,
            created_by_id: row.created_by_id,
        }));
    }
    if kinds.contains(&MovementKind::Out) {
        let models = scope_allowed::<super::routes::InventoryView, _>(OutEntity::find())
            .all(db)
            .await
            .unwrap_or_default();
        rows.extend(models.into_iter().map(|row| ListedMovement {
            id: row.id,
            kind: MovementKind::Out,
            number: row.number,
            datetime: row.datetime,
            bill_to: Some(BillTo::from_row(
                row.bill_to_individual,
                row.customer_individual,
                row.customer_company,
            )),
            vehicle_type: row.vehicle_type,
            vehicle_number: row.vehicle_number,
            driver_id: row.driver_id,
            created_at: row.created_at,
            created_by_id: row.created_by_id,
        }));
    }
    if kinds.contains(&MovementKind::Process) {
        let models = scope_allowed::<super::routes::InventoryView, _>(ProcessEntity::find())
            .all(db)
            .await
            .unwrap_or_default();
        rows.extend(models.into_iter().map(|row| ListedMovement {
            id: row.id,
            kind: MovementKind::Process,
            number: row.number,
            datetime: row.datetime,
            bill_to: None,
            vehicle_type: None,
            vehicle_number: None,
            driver_id: None,
            created_at: row.created_at,
            created_by_id: row.created_by_id,
        }));
    }
    rows
}

fn movement_display(id: i64, number: &str) -> String {
    let trimmed = number.trim();
    if trimmed.is_empty() {
        format!("#{id}")
    } else {
        trimmed.to_string()
    }
}

pub async fn in_movement_labels(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    InEntity::find()
        .filter(stock_movement_in::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.id, movement_display(row.id, &row.number)))
        .collect()
}

pub async fn process_movement_labels(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    ProcessEntity::find()
        .filter(stock_movement_process::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.id, movement_display(row.id, &row.number)))
        .collect()
}

pub fn sort_movements(rows: &mut [ListedMovement], sort: &str) {
    let desc = sort_desc(sort);
    match sort_key(sort) {
        key if key.eq_ignore_ascii_case("Number") => {
            rows.sort_by(|a, b| cmp_text(&a.number, &b.number, desc));
        }
        key if key.eq_ignore_ascii_case("Type") => {
            rows.sort_by(|a, b| cmp_text(a.kind.label(), b.kind.label(), desc));
        }
        key if key.eq_ignore_ascii_case("CreatedAt") => {
            rows.sort_by(|a, b| {
                let ord = a.created_at.cmp(&b.created_at);
                if desc { ord.reverse() } else { ord }
            });
        }
        _ => rows.sort_by(|a, b| {
            let ord = a.datetime.cmp(&b.datetime);
            if desc { ord.reverse() } else { ord }
        }),
    }
}

fn cmp_text(a: &str, b: &str, desc: bool) -> std::cmp::Ordering {
    let ord = a.cmp(b);
    if desc { ord.reverse() } else { ord }
}

pub fn slice_page<T>(items: Vec<T>, page: u32, page_size: u32) -> (Vec<T>, u32, u64) {
    let total = items.len() as u64;
    let page_size = page_size.max(1) as usize;
    let start = (page.saturating_sub(1) as usize).saturating_mul(page_size);
    let rows = items.into_iter().skip(start).take(page_size).collect();
    (rows, page, total)
}

pub async fn company_name(db: &DatabaseConnection, id: i64) -> String {
    let names = company_names(db, &[id]).await;
    names.get(&id).cloned().unwrap_or_default()
}

pub async fn company_names(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    let rows = CompanyEntity::find()
        .filter(company::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default();
    rows.into_iter().map(|row| (row.id, row.name)).collect()
}

pub async fn stock_names(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    let rows = StockEntity::find()
        .filter(stock::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default();
    rows.into_iter().map(|row| (row.id, row.name)).collect()
}

pub async fn on_hand_for_stocks(
    db: &DatabaseConnection,
    stocks: &[stock::Model],
) -> HashMap<i64, Result<Decimal, String>> {
    let mut out = HashMap::new();
    if stocks.is_empty() {
        return out;
    }
    let ids: Vec<i64> = stocks.iter().map(|s| s.id).collect();
    let mut by_stock: HashMap<i64, Vec<QtyLine>> = HashMap::new();
    let in_lines = InLineEntity::find()
        .filter(stock_movement_in_line::Column::StockId.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default();
    for line in in_lines {
        by_stock.entry(line.stock_id).or_default().push(QtyLine {
            sign: QtySign::Add,
            qty: line.qty,
            qty_unit: line.qty_unit,
            qty_type: line.qty_type,
        });
    }
    let out_lines = OutLineEntity::find()
        .filter(stock_movement_out_line::Column::StockId.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default();
    for line in out_lines {
        by_stock.entry(line.stock_id).or_default().push(QtyLine {
            sign: QtySign::Subtract,
            qty: line.qty,
            qty_unit: line.qty_unit,
            qty_type: line.qty_type,
        });
    }
    let process_lines = ProcessLineEntity::find()
        .filter(stock_movement_process_line::Column::StockId.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default();
    for line in process_lines {
        let sign = match line.side {
            ProcessLineSide::Input => QtySign::Add,
            ProcessLineSide::Output => QtySign::Subtract,
        };
        by_stock.entry(line.stock_id).or_default().push(QtyLine {
            sign,
            qty: line.qty,
            qty_unit: line.qty_unit,
            qty_type: line.qty_type,
        });
    }
    for stock in stocks {
        let qty_lines = by_stock.remove(&stock.id).unwrap_or_default();
        out.insert(
            stock.id,
            sum_on_hand(&stock.qty_type, &stock.qty_unit, &qty_lines),
        );
    }
    out
}

pub async fn on_hand(db: &DatabaseConnection, stock: &stock::Model) -> Result<Decimal, String> {
    let map = on_hand_for_stocks(db, std::slice::from_ref(stock)).await;
    match map.get(&stock.id) {
        Some(result) => result.clone(),
        None => Ok(Decimal::ZERO),
    }
}

fn sort_key(sort: &str) -> &str {
    sort.split_whitespace().next().unwrap_or("")
}

fn sort_desc(sort: &str) -> bool {
    sort.split_whitespace()
        .last()
        .is_some_and(|dir| dir.eq_ignore_ascii_case("DESC"))
}

pub fn apply_stock_sort(query: Select<StockEntity>, sort: &str) -> Select<StockEntity> {
    let desc = sort_desc(sort);
    match sort_key(sort) {
        key if key.eq_ignore_ascii_case("Name") => {
            if desc {
                query.order_by_desc(stock::Column::Name)
            } else {
                query.order_by_asc(stock::Column::Name)
            }
        }
        _ => query.order_by_asc(stock::Column::Name),
    }
}

pub async fn page_of<E: EntityTrait>(
    query: Select<E>,
    db: &DatabaseConnection,
    page: u32,
    page_size: u32,
) -> (Vec<E::Model>, u32, u64)
where
    E::Model: Sync,
{
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}
