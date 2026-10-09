use std::collections::HashMap;

use lariv_plugin_contacts::entities::company::{self, Entity as CompanyEntity};
use lariv_plugin_users::role_authorization::scope_allowed;
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Select,
};

use crate::entities::{
    stock::{self, Entity as StockEntity},
    stock_movement::{self, Entity as MovementEntity},
    stock_movement_line::{self, Entity as LineEntity},
};
use crate::logic::qty::{QtyLine, sum_on_hand};
use crate::movement_type::MovementType;

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

pub async fn find_movement(db: &DatabaseConnection, id: i64) -> Option<stock_movement::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::InventoryView, _>(MovementEntity::find_by_id(id))
            .one(db)
            .await,
        "find movement",
    )
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
    let lines = LineEntity::find()
        .filter(stock_movement_line::Column::StockId.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default();
    let movement_ids: Vec<i64> = lines.iter().map(|line| line.stock_movement_id).collect();
    let movements = if movement_ids.is_empty() {
        Vec::new()
    } else {
        MovementEntity::find()
            .filter(stock_movement::Column::Id.is_in(movement_ids))
            .all(db)
            .await
            .unwrap_or_default()
    };
    let movement_types: HashMap<i64, MovementType> = movements
        .into_iter()
        .map(|row| (row.id, row.movement_type))
        .collect();
    let mut by_stock: HashMap<i64, Vec<&stock_movement_line::Model>> = HashMap::new();
    for line in &lines {
        by_stock.entry(line.stock_id).or_default().push(line);
    }
    for stock in stocks {
        let qty_lines: Vec<QtyLine> = by_stock
            .get(&stock.id)
            .map(|rows| {
                rows.iter()
                    .filter_map(|line| {
                        let movement = movement_types.get(&line.stock_movement_id).copied()?;
                        Some(QtyLine {
                            movement,
                            qty: line.qty,
                            qty_unit: line.qty_unit.clone(),
                            qty_type: line.qty_type.clone(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
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

pub fn apply_movement_sort(query: Select<MovementEntity>, sort: &str) -> Select<MovementEntity> {
    let desc = sort_desc(sort);
    match sort_key(sort) {
        key if key.eq_ignore_ascii_case("Datetime") => {
            if desc {
                query.order_by_desc(stock_movement::Column::Datetime)
            } else {
                query.order_by_asc(stock_movement::Column::Datetime)
            }
        }
        key if key.eq_ignore_ascii_case("Type") => {
            if desc {
                query.order_by_desc(stock_movement::Column::MovementType)
            } else {
                query.order_by_asc(stock_movement::Column::MovementType)
            }
        }
        key if key.eq_ignore_ascii_case("Number") => {
            if desc {
                query.order_by_desc(stock_movement::Column::Number)
            } else {
                query.order_by_asc(stock_movement::Column::Number)
            }
        }
        _ => query.order_by_desc(stock_movement::Column::Datetime),
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
