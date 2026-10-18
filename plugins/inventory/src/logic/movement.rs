use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait,
};

use crate::entities::stock::Entity as StockEntity;
use crate::entities::stock_movement_in::{self, Entity as InEntity};
use crate::entities::stock_movement_in_line::{self, Entity as InLineEntity};
use crate::entities::stock_movement_out::{self, Entity as OutEntity};
use crate::entities::stock_movement_out_in::{self, Entity as OutInEntity};
use crate::entities::stock_movement_out_process::{self, Entity as OutProcessEntity};
use crate::entities::stock_movement_process::{self, Entity as ProcessEntity};
use crate::entities::stock_movement_process_line::{
    self, Entity as ProcessLineEntity, ProcessLineSide,
};
use crate::logic::line::{
    LineInput, PreparedLine, delete_in_lines, delete_out_lines, delete_process_lines,
    insert_in_lines, insert_out_lines, insert_process_lines, prepare_lines,
};
use crate::logic::number::next_movement_number;
use crate::logic::preferences::parse_optional_id;
use crate::logic::qty::{QtyLine, QtySign, sum_on_hand};
use crate::movement_type::MovementKind;
use crate::scope::{find_movement_in, find_movement_out, find_movement_process, stock_names};
use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;

/// In movements and process movements an out movement draws stock from.
#[derive(Clone, Debug, Default)]
pub struct OutSources {
    pub in_ids: Vec<i64>,
    pub process_ids: Vec<i64>,
}

impl OutSources {
    pub fn is_empty(&self) -> bool {
        self.in_ids.is_empty() && self.process_ids.is_empty()
    }
}

pub struct PartyInput {
    pub created_by_id: i64,
    pub number: String,
    pub datetime: DateTime<Utc>,
    pub vehicle_out_datetime: Option<DateTime<Utc>>,
    pub customer_id: i64,
    pub vehicle_type: Option<String>,
    pub vehicle_number: Option<String>,
    pub eway_bill: Option<String>,
    pub driver_id: Option<i64>,
    pub delivery_challan: Option<i64>,
    /// In and process movements this out draws from. Required for out movements.
    pub sources: Option<OutSources>,
    pub lines: Vec<LineInput>,
}

pub struct ProcessInput {
    pub created_by_id: i64,
    pub number: String,
    pub datetime: DateTime<Utc>,
    pub customer_id: i64,
    pub inputs: Vec<LineInput>,
    pub outputs: Vec<LineInput>,
}

fn positive_id(id: i64) -> Option<i64> {
    Some(id).filter(|id| *id > 0)
}

fn optional_text(raw: Option<String>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn dedupe_ids(ids: &[i64]) -> Vec<i64> {
    let mut out = Vec::new();
    for id in ids {
        if *id > 0 && !out.contains(id) {
            out.push(*id);
        }
    }
    out
}

pub async fn require_out_sources(
    db: &sea_orm::DatabaseConnection,
    in_ids: &[i64],
    process_ids: &[i64],
) -> Result<OutSources, String> {
    let in_ids = dedupe_ids(in_ids);
    let process_ids = dedupe_ids(process_ids);
    if in_ids.is_empty() && process_ids.is_empty() {
        return Err("select an in or process movement".into());
    }
    for id in &in_ids {
        if find_movement_in(db, *id).await.is_none() {
            return Err("in movement not found".into());
        }
    }
    for id in &process_ids {
        if find_movement_process(db, *id).await.is_none() {
            return Err("process movement not found".into());
        }
    }
    Ok(OutSources {
        in_ids,
        process_ids,
    })
}

/// Stock ids present on the selected in lines and process output lines.
pub async fn source_stock_ids(
    db: &sea_orm::DatabaseConnection,
    sources: &OutSources,
) -> Result<Vec<i64>, String> {
    let mut ids = HashSet::new();
    if !sources.in_ids.is_empty() {
        let lines = InLineEntity::find()
            .filter(stock_movement_in_line::Column::MovementId.is_in(sources.in_ids.clone()))
            .all(db)
            .await
            .map_err(|err| err.to_string())?;
        ids.extend(lines.into_iter().map(|line| line.stock_id));
    }
    if !sources.process_ids.is_empty() {
        let lines = ProcessLineEntity::find()
            .filter(
                stock_movement_process_line::Column::MovementId.is_in(sources.process_ids.clone()),
            )
            .filter(stock_movement_process_line::Column::Side.eq(ProcessLineSide::Output))
            .all(db)
            .await
            .map_err(|err| err.to_string())?;
        ids.extend(lines.into_iter().map(|line| line.stock_id));
    }
    let mut ids: Vec<i64> = ids.into_iter().collect();
    ids.sort_unstable();
    Ok(ids)
}

pub async fn out_sources_for(
    db: &sea_orm::DatabaseConnection,
    out_ids: &[i64],
) -> HashMap<i64, OutSources> {
    let mut map: HashMap<i64, OutSources> = HashMap::new();
    if out_ids.is_empty() {
        return map;
    }
    let ins = OutInEntity::find()
        .filter(stock_movement_out_in::Column::OutId.is_in(out_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    for row in ins {
        map.entry(row.out_id).or_default().in_ids.push(row.in_id);
    }
    let processes = OutProcessEntity::find()
        .filter(stock_movement_out_process::Column::OutId.is_in(out_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    for row in processes {
        map.entry(row.out_id)
            .or_default()
            .process_ids
            .push(row.process_id);
    }
    map
}

async fn replace_out_sources<C: ConnectionTrait>(
    db: &C,
    out_id: i64,
    sources: &OutSources,
) -> Result<(), String> {
    OutInEntity::delete_many()
        .filter(stock_movement_out_in::Column::OutId.eq(out_id))
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    OutProcessEntity::delete_many()
        .filter(stock_movement_out_process::Column::OutId.eq(out_id))
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    for in_id in &sources.in_ids {
        stock_movement_out_in::ActiveModel {
            out_id: Set(out_id),
            in_id: Set(*in_id),
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    for process_id in &sources.process_ids {
        stock_movement_out_process::ActiveModel {
            out_id: Set(out_id),
            process_id: Set(*process_id),
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn qty_line(line: &PreparedLine) -> QtyLine {
    QtyLine {
        sign: QtySign::Add,
        qty: line.qty,
        qty_unit: line.qty_unit.clone(),
        qty_type: line.qty_type.clone(),
    }
}

fn qty_line_stored(qty: Decimal, qty_unit: &str, qty_type: &str) -> QtyLine {
    QtyLine {
        sign: QtySign::Add,
        qty,
        qty_unit: qty_unit.to_string(),
        qty_type: qty_type.to_string(),
    }
}

/// Out lines may only use stock, and no more of it, than the selected movements hold.
pub async fn ensure_out_within_sources(
    db: &sea_orm::DatabaseConnection,
    sources: &OutSources,
    lines: &[PreparedLine],
) -> Result<(), String> {
    if sources.is_empty() {
        return Err("select an in or process movement".into());
    }
    let mut available: HashMap<i64, Vec<QtyLine>> = HashMap::new();
    if !sources.in_ids.is_empty() {
        let rows = InLineEntity::find()
            .filter(stock_movement_in_line::Column::MovementId.is_in(sources.in_ids.clone()))
            .all(db)
            .await
            .map_err(|err| err.to_string())?;
        for row in rows {
            available
                .entry(row.stock_id)
                .or_default()
                .push(qty_line_stored(row.qty, &row.qty_unit, &row.qty_type));
        }
    }
    if !sources.process_ids.is_empty() {
        let rows = ProcessLineEntity::find()
            .filter(
                stock_movement_process_line::Column::MovementId.is_in(sources.process_ids.clone()),
            )
            .filter(stock_movement_process_line::Column::Side.eq(ProcessLineSide::Output))
            .all(db)
            .await
            .map_err(|err| err.to_string())?;
        for row in rows {
            available
                .entry(row.stock_id)
                .or_default()
                .push(qty_line_stored(row.qty, &row.qty_unit, &row.qty_type));
        }
    }
    let mut taken: HashMap<i64, Vec<QtyLine>> = HashMap::new();
    for line in lines {
        taken.entry(line.stock_id).or_default().push(qty_line(line));
    }
    let mut stock_ids: Vec<i64> = taken.keys().copied().collect();
    stock_ids.sort_unstable();
    let names = stock_names(db, &stock_ids).await;
    let stocks = StockEntity::find()
        .filter(crate::entities::stock::Column::Id.is_in(stock_ids.clone()))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let stocks: HashMap<i64, _> = stocks.into_iter().map(|row| (row.id, row)).collect();
    for stock_id in stock_ids {
        let name = names
            .get(&stock_id)
            .cloned()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("#{stock_id}"));
        let Some(source_lines) = available.get(&stock_id) else {
            return Err(format!("{name} is not on the selected movements"));
        };
        let Some(stock) = stocks.get(&stock_id) else {
            return Err(format!("{name} was not found"));
        };
        let have = sum_on_hand(&stock.qty_type, &stock.qty_unit, source_lines)?;
        let want = sum_on_hand(
            &stock.qty_type,
            &stock.qty_unit,
            taken.get(&stock_id).map(Vec::as_slice).unwrap_or(&[]),
        )?;
        if want > have {
            return Err(format!("{name} is more than the selected movements have"));
        }
    }
    Ok(())
}

pub async fn require_challan(
    db: &sea_orm::DatabaseConnection,
    raw: &str,
) -> Result<Option<i64>, String> {
    let Some(id) = parse_optional_id(raw) else {
        return Ok(None);
    };
    let node = VNodeEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    let Some(node) = node else {
        return Err("delivery challan file not found".into());
    };
    if node.is_directory {
        return Err("delivery challan must be a file".into());
    }
    Ok(Some(id))
}

pub async fn create_in(
    db: &sea_orm::DatabaseConnection,
    input: PartyInput,
) -> Result<stock_movement_in::Model, String> {
    let prepared = prepare_lines(db, &input.lines).await?;
    let number = next_movement_number(db, input.datetime, MovementKind::In, &input.number).await?;
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    let model = stock_movement_in::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        datetime: Set(input.datetime),
        vehicle_out_datetime: Set(input.vehicle_out_datetime),
        number: Set(number),
        customer_id: Set(input.customer_id),
        vehicle_type: Set(optional_text(input.vehicle_type)),
        vehicle_number: Set(optional_text(input.vehicle_number)),
        eway_bill: Set(optional_text(input.eway_bill)),
        driver_id: Set(input.driver_id.filter(|id| *id > 0)),
        delivery_challan: Set(input.delivery_challan.filter(|id| *id > 0)),
        created_by_id: Set(positive_id(input.created_by_id)),
    }
    .insert(&txn)
    .await
    .map_err(|err| err.to_string())?;
    insert_in_lines(&txn, model.id, &prepared).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(model)
}

pub async fn update_in(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    input: PartyInput,
) -> Result<stock_movement_in::Model, String> {
    let existing = find_movement_in(db, id)
        .await
        .ok_or_else(|| "movement not found".to_string())?;
    let prepared = prepare_lines(db, &input.lines).await?;
    let number = next_movement_number(db, input.datetime, MovementKind::In, &input.number).await?;
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    let mut am: stock_movement_in::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.datetime = Set(input.datetime);
    am.vehicle_out_datetime = Set(input.vehicle_out_datetime);
    am.number = Set(number);
    am.customer_id = Set(input.customer_id);
    am.vehicle_type = Set(optional_text(input.vehicle_type));
    am.vehicle_number = Set(optional_text(input.vehicle_number));
    am.eway_bill = Set(optional_text(input.eway_bill));
    am.driver_id = Set(input.driver_id.filter(|id| *id > 0));
    am.delivery_challan = Set(input.delivery_challan.filter(|id| *id > 0));
    let model = am.update(&txn).await.map_err(|err| err.to_string())?;
    delete_in_lines(&txn, model.id).await?;
    insert_in_lines(&txn, model.id, &prepared).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(model)
}

pub async fn delete_in(db: &sea_orm::DatabaseConnection, id: i64) -> Result<(), String> {
    let existing = find_movement_in(db, id)
        .await
        .ok_or_else(|| "movement not found".to_string())?;
    let used = OutInEntity::find()
        .filter(stock_movement_out_in::Column::InId.eq(existing.id))
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    if used.is_some() {
        return Err("movement is used by an out movement".into());
    }
    InEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub async fn create_out(
    db: &sea_orm::DatabaseConnection,
    input: PartyInput,
) -> Result<stock_movement_out::Model, String> {
    let sources = input
        .sources
        .ok_or_else(|| "select an in or process movement".to_string())?;
    let prepared = prepare_lines(db, &input.lines).await?;
    ensure_out_within_sources(db, &sources, &prepared).await?;
    let number = next_movement_number(db, input.datetime, MovementKind::Out, &input.number).await?;
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    let model = stock_movement_out::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        datetime: Set(input.datetime),
        vehicle_out_datetime: Set(input.vehicle_out_datetime),
        number: Set(number),
        customer_id: Set(input.customer_id),
        vehicle_type: Set(optional_text(input.vehicle_type)),
        vehicle_number: Set(optional_text(input.vehicle_number)),
        eway_bill: Set(optional_text(input.eway_bill)),
        driver_id: Set(input.driver_id.filter(|id| *id > 0)),
        created_by_id: Set(positive_id(input.created_by_id)),
    }
    .insert(&txn)
    .await
    .map_err(|err| err.to_string())?;
    replace_out_sources(&txn, model.id, &sources).await?;
    insert_out_lines(&txn, model.id, &prepared).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(model)
}

pub async fn update_out(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    input: PartyInput,
) -> Result<stock_movement_out::Model, String> {
    let sources = input
        .sources
        .ok_or_else(|| "select an in or process movement".to_string())?;
    let existing = find_movement_out(db, id)
        .await
        .ok_or_else(|| "movement not found".to_string())?;
    let prepared = prepare_lines(db, &input.lines).await?;
    ensure_out_within_sources(db, &sources, &prepared).await?;
    let number = next_movement_number(db, input.datetime, MovementKind::Out, &input.number).await?;
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    let mut am: stock_movement_out::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.datetime = Set(input.datetime);
    am.vehicle_out_datetime = Set(input.vehicle_out_datetime);
    am.number = Set(number);
    am.customer_id = Set(input.customer_id);
    am.vehicle_type = Set(optional_text(input.vehicle_type));
    am.vehicle_number = Set(optional_text(input.vehicle_number));
    am.eway_bill = Set(optional_text(input.eway_bill));
    am.driver_id = Set(input.driver_id.filter(|id| *id > 0));
    let model = am.update(&txn).await.map_err(|err| err.to_string())?;
    replace_out_sources(&txn, model.id, &sources).await?;
    delete_out_lines(&txn, model.id).await?;
    insert_out_lines(&txn, model.id, &prepared).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(model)
}

pub async fn delete_out(db: &sea_orm::DatabaseConnection, id: i64) -> Result<(), String> {
    let existing = find_movement_out(db, id)
        .await
        .ok_or_else(|| "movement not found".to_string())?;
    OutInEntity::delete_many()
        .filter(stock_movement_out_in::Column::OutId.eq(existing.id))
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    OutProcessEntity::delete_many()
        .filter(stock_movement_out_process::Column::OutId.eq(existing.id))
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    OutEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub async fn create_process(
    db: &sea_orm::DatabaseConnection,
    input: ProcessInput,
) -> Result<stock_movement_process::Model, String> {
    let inputs = prepare_lines(db, &input.inputs).await?;
    let outputs = prepare_lines(db, &input.outputs).await?;
    let number =
        next_movement_number(db, input.datetime, MovementKind::Process, &input.number).await?;
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    let model = stock_movement_process::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        datetime: Set(input.datetime),
        number: Set(number),
        customer_id: Set(input.customer_id),
        created_by_id: Set(positive_id(input.created_by_id)),
    }
    .insert(&txn)
    .await
    .map_err(|err| err.to_string())?;
    insert_process_lines(&txn, model.id, ProcessLineSide::Input, &inputs).await?;
    insert_process_lines(&txn, model.id, ProcessLineSide::Output, &outputs).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(model)
}

pub async fn update_process(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    input: ProcessInput,
) -> Result<stock_movement_process::Model, String> {
    let existing = find_movement_process(db, id)
        .await
        .ok_or_else(|| "movement not found".to_string())?;
    let inputs = prepare_lines(db, &input.inputs).await?;
    let outputs = prepare_lines(db, &input.outputs).await?;
    let number =
        next_movement_number(db, input.datetime, MovementKind::Process, &input.number).await?;
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    let mut am: stock_movement_process::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.datetime = Set(input.datetime);
    am.number = Set(number);
    am.customer_id = Set(input.customer_id);
    let model = am.update(&txn).await.map_err(|err| err.to_string())?;
    delete_process_lines(&txn, model.id).await?;
    insert_process_lines(&txn, model.id, ProcessLineSide::Input, &inputs).await?;
    insert_process_lines(&txn, model.id, ProcessLineSide::Output, &outputs).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(model)
}

pub async fn delete_process(db: &sea_orm::DatabaseConnection, id: i64) -> Result<(), String> {
    let existing = find_movement_process(db, id)
        .await
        .ok_or_else(|| "movement not found".to_string())?;
    let used = OutProcessEntity::find()
        .filter(stock_movement_out_process::Column::ProcessId.eq(existing.id))
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    if used.is_some() {
        return Err("movement is used by an out movement".into());
    }
    ProcessEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
