//! Create, update, copy, and delete quotations and their lines.

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::Deserialize;

use crate::entities::{
    quotation::{self, Entity as QuotationEntity},
    quotation_goods_line::{self, Entity as GoodsLineEntity},
    quotation_service_line::{self, Entity as ServiceLineEntity},
};
use crate::logic::number::resolve_quotation_number;
use crate::logic::prefs::load_preferences;
use crate::logic::tax::{set_goods_line_taxes, set_service_line_taxes};
use lariv_plugin_finance_common::decimal::{self, parse_decimal};
use lariv_plugin_finance_products::{
    entities::product::Entity as ProductEntity, preferences::load_product_tax_ids,
    pricing as product_pricing,
};
use lariv_plugin_finance_services::{
    entities::service::Entity as ServiceEntity, preferences::load_service_tax_ids,
    pricing as service_pricing,
};
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;

#[derive(Debug, Clone, Deserialize)]
pub struct GoodsLineInput {
    pub product_id: i64,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default = "default_quantity")]
    pub quantity: String,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(default)]
    pub tax_ids: Option<Vec<i64>>,
    #[serde(default)]
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceLineInput {
    pub service_id: i64,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default = "default_quantity")]
    pub quantity: String,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(default)]
    pub tax_ids: Option<Vec<i64>>,
    #[serde(default)]
    pub remarks: Option<String>,
}

fn default_quantity() -> String {
    "1".to_string()
}

pub fn parse_goods_lines(raw: &str) -> Result<Vec<GoodsLineInput>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(raw).map_err(|err| format!("invalid goods lines data: {err}"))
}

pub fn parse_service_lines(raw: &str) -> Result<Vec<ServiceLineInput>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(raw).map_err(|err| format!("invalid service lines data: {err}"))
}

pub fn parse_quotation_date(raw: &str) -> Result<NaiveDate, String> {
    lariv_core::datetime::parse_date(raw.trim()).ok_or_else(|| "invalid date".to_string())
}

fn optional_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn line_is_blank(id: i64, rate: &Option<String>, remarks: &Option<String>) -> bool {
    id <= 0
        && rate.as_deref().unwrap_or("").trim().is_empty()
        && remarks.as_deref().unwrap_or("").trim().is_empty()
}

fn occupied_goods(lines: &[GoodsLineInput]) -> Result<Vec<GoodsLineInput>, String> {
    let mut out = Vec::new();
    for row in lines {
        if line_is_blank(row.product_id, &row.rate, &row.remarks) {
            continue;
        }
        if row.product_id <= 0 {
            return Err("choose a product for each goods line".to_string());
        }
        out.push(row.clone());
    }
    Ok(out)
}

fn occupied_services(lines: &[ServiceLineInput]) -> Result<Vec<ServiceLineInput>, String> {
    let mut out = Vec::new();
    for row in lines {
        if line_is_blank(row.service_id, &row.rate, &row.remarks) {
            continue;
        }
        if row.service_id <= 0 {
            return Err("choose a service for each service line".to_string());
        }
        out.push(row.clone());
    }
    Ok(out)
}

fn optional_rate(raw: &Option<String>) -> Result<Option<Decimal>, String> {
    let Some(raw) = raw.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let rate = parse_decimal(raw).ok_or_else(|| "invalid rate".to_string())?;
    if rate < Decimal::ZERO {
        return Err("rate must be non-negative".to_string());
    }
    Ok(Some(rate))
}

fn parse_quantity(raw: &str) -> Result<Decimal, String> {
    let raw = if raw.trim().is_empty() {
        "1"
    } else {
        raw.trim()
    };
    parse_decimal(raw)
        .filter(|qty| *qty > Decimal::ZERO)
        .ok_or_else(|| "quantity must be positive".to_string())
}

fn line_taxes(explicit: &Option<Vec<i64>>, catalog: &[i64]) -> Vec<i64> {
    explicit
        .as_ref()
        .cloned()
        .unwrap_or_else(|| catalog.to_vec())
}

async fn validate_tax_ids(db: &DatabaseConnection, tax_ids: &[i64]) -> Result<(), String> {
    if tax_ids.is_empty() {
        return Ok(());
    }
    let loaded = load_taxes_by_ids(db, tax_ids)
        .await
        .map_err(|err| err.to_string())?;
    if loaded.len() != tax_ids.len() {
        return Err("one or more line tax ids are invalid".to_string());
    }
    Ok(())
}

struct Priced {
    rate: Decimal,
    quantity: Decimal,
    pre_tax: Decimal,
    variable_values: String,
    remarks: Option<String>,
    tax_ids: Vec<i64>,
}

async fn price_goods(db: &DatabaseConnection, row: &GoodsLineInput) -> Result<Priced, String> {
    let product = ProductEntity::find_by_id(row.product_id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("unknown product #{}", row.product_id))?;
    let quantity = parse_quantity(&row.quantity)?;
    let explicit = optional_rate(&row.rate)?;
    let priced = if product_pricing::has_variable_pricing(&product.variables) {
        let raw = row
            .variables
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));
        let unit =
            product_pricing::price_line(&product.variables, &product.sales_price_formula, &raw)?;
        let mut priced = product_pricing::with_quantity(unit, quantity)?;
        if let Some(rate) = explicit {
            priced.rate = decimal::normalize(rate);
            priced.pre_tax = decimal::dec_mul(priced.quantity, priced.rate);
        }
        priced
    } else {
        let rate = match explicit {
            Some(rate) => rate,
            None => product_pricing::unit_amount(&product.sales_price_formula)?,
        };
        product_pricing::from_quantity_rate(quantity, rate)?
    };
    let catalog = load_product_tax_ids(db, product.id).await;
    let tax_ids = line_taxes(&row.tax_ids, &catalog);
    validate_tax_ids(db, &tax_ids).await?;
    let remarks = match &row.remarks {
        Some(text) => optional_text(text),
        None => product.remarks.as_deref().and_then(optional_text),
    };
    Ok(Priced {
        rate: priced.rate,
        quantity: priced.quantity,
        pre_tax: priced.pre_tax,
        variable_values: priced.variable_values,
        remarks,
        tax_ids,
    })
}

async fn price_service(db: &DatabaseConnection, row: &ServiceLineInput) -> Result<Priced, String> {
    let service = ServiceEntity::find_by_id(row.service_id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("unknown service #{}", row.service_id))?;
    let quantity = parse_quantity(&row.quantity)?;
    let explicit = optional_rate(&row.rate)?;
    let priced = if service_pricing::has_variable_pricing(&service.variables) {
        let raw = row
            .variables
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));
        let unit =
            service_pricing::price_line(&service.variables, &service.sales_price_formula, &raw)?;
        let mut priced = service_pricing::with_quantity(unit, quantity)?;
        if let Some(rate) = explicit {
            priced.rate = decimal::normalize(rate);
            priced.pre_tax = decimal::dec_mul(priced.quantity, priced.rate);
        }
        priced
    } else {
        let rate = match explicit {
            Some(rate) => rate,
            None => service_pricing::unit_amount(&service.sales_price_formula)?,
        };
        service_pricing::from_quantity_rate(quantity, rate)?
    };
    let catalog = load_service_tax_ids(db, service.id).await;
    let tax_ids = line_taxes(&row.tax_ids, &catalog);
    validate_tax_ids(db, &tax_ids).await?;
    let remarks = match &row.remarks {
        Some(text) => optional_text(text),
        None => service.remarks.as_deref().and_then(optional_text),
    };
    Ok(Priced {
        rate: priced.rate,
        quantity: priced.quantity,
        pre_tax: priced.pre_tax,
        variable_values: priced.variable_values,
        remarks,
        tax_ids,
    })
}

async fn insert_goods<C: ConnectionTrait>(
    db: &DatabaseConnection,
    txn: &C,
    quotation_id: i64,
    row: &GoodsLineInput,
) -> Result<(), String> {
    let priced = price_goods(db, row).await?;
    let now = Utc::now();
    let line = quotation_goods_line::ActiveModel {
        quotation_id: Set(quotation_id),
        product_id: Set(row.product_id),
        rate: Set(priced.rate),
        quantity: Set(priced.quantity),
        variable_values: Set(priced.variable_values),
        pre_tax_amount: Set(priced.pre_tax),
        remarks: Set(priced.remarks),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(txn)
    .await
    .map_err(|err| err.to_string())?;
    set_goods_line_taxes(txn, line.id, &priced.tax_ids)
        .await
        .map_err(|err| err.to_string())
}

async fn insert_service<C: ConnectionTrait>(
    db: &DatabaseConnection,
    txn: &C,
    quotation_id: i64,
    row: &ServiceLineInput,
) -> Result<(), String> {
    let priced = price_service(db, row).await?;
    let now = Utc::now();
    let line = quotation_service_line::ActiveModel {
        quotation_id: Set(quotation_id),
        service_id: Set(row.service_id),
        rate: Set(priced.rate),
        quantity: Set(priced.quantity),
        variable_values: Set(priced.variable_values),
        pre_tax_amount: Set(priced.pre_tax),
        remarks: Set(priced.remarks),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(txn)
    .await
    .map_err(|err| err.to_string())?;
    set_service_line_taxes(txn, line.id, &priced.tax_ids)
        .await
        .map_err(|err| err.to_string())
}

pub struct SaveQuotation {
    pub existing_id: Option<i64>,
    pub number: String,
    pub date: NaiveDate,
    pub customer_id: i64,
    pub bank_account: String,
    pub remarks: String,
    pub goods: Vec<GoodsLineInput>,
    pub services: Vec<ServiceLineInput>,
}

pub async fn create_quotation(
    db: &DatabaseConnection,
    input: SaveQuotation,
) -> Result<quotation::Model, String> {
    save(db, None, input).await
}

pub async fn update_quotation(
    db: &DatabaseConnection,
    id: i64,
    input: SaveQuotation,
) -> Result<quotation::Model, String> {
    save(db, Some(id), input).await
}

async fn save(
    db: &DatabaseConnection,
    existing_id: Option<i64>,
    input: SaveQuotation,
) -> Result<quotation::Model, String> {
    let goods = occupied_goods(&input.goods)?;
    let services = occupied_services(&input.services)?;
    if goods.is_empty() && services.is_empty() {
        return Err("add at least one goods or service line".to_string());
    }
    let customer_id =
        lariv_plugin_crm::logic::customer::require_existing(db, input.customer_id).await?;
    let number = resolve_quotation_number(db, existing_id, input.date, &input.number).await?;
    let mut bank = optional_text(&input.bank_account);
    if bank.is_none() && existing_id.is_none() {
        bank = load_preferences(db).await.default_bank_account;
    }
    let remarks = input.remarks.trim().to_string();
    let now = Utc::now();

    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let saved = if let Some(id) = existing_id {
        let mut am: quotation::ActiveModel = QuotationEntity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| "quotation not found".to_string())?
            .into();
        am.number = Set(number);
        am.date = Set(input.date);
        am.customer_id = Set(customer_id);
        am.bank_account = Set(bank);
        am.remarks = Set(remarks);
        am.updated_at = Set(Some(now));
        let saved = am.update(&txn).await.map_err(|err| err.to_string())?;
        GoodsLineEntity::delete_many()
            .filter(quotation_goods_line::Column::QuotationId.eq(id))
            .exec(&txn)
            .await
            .map_err(|err| err.to_string())?;
        ServiceLineEntity::delete_many()
            .filter(quotation_service_line::Column::QuotationId.eq(id))
            .exec(&txn)
            .await
            .map_err(|err| err.to_string())?;
        saved
    } else {
        quotation::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            date: Set(input.date),
            customer_id: Set(customer_id),
            number: Set(number),
            bank_account: Set(bank),
            remarks: Set(remarks),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|err| err.to_string())?
    };

    for row in &goods {
        insert_goods(db, &txn, saved.id, row).await?;
    }
    for row in &services {
        insert_service(db, &txn, saved.id, row).await?;
    }
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(saved)
}

pub async fn delete_quotation(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    QuotationEntity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub async fn copy_quotation(db: &DatabaseConnection, id: i64) -> Result<quotation::Model, String> {
    let source = QuotationEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "quotation not found".to_string())?;
    let goods = GoodsLineEntity::find()
        .filter(quotation_goods_line::Column::QuotationId.eq(id))
        .order_by_asc(quotation_goods_line::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let services = ServiceLineEntity::find()
        .filter(quotation_service_line::Column::QuotationId.eq(id))
        .order_by_asc(quotation_service_line::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let date = crate::logic::number::today_utc();
    let number = resolve_quotation_number(db, None, date, "").await?;
    let now = Utc::now();
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let saved = quotation::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        date: Set(date),
        customer_id: Set(source.customer_id),
        number: Set(number),
        bank_account: Set(source.bank_account.clone()),
        remarks: Set(source.remarks.clone()),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|err| err.to_string())?;

    for line in goods {
        let tax_ids = crate::logic::tax::load_goods_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let copied = quotation_goods_line::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            quotation_id: Set(saved.id),
            product_id: Set(line.product_id),
            rate: Set(line.rate),
            quantity: Set(line.quantity),
            variable_values: Set(line.variable_values),
            pre_tax_amount: Set(line.pre_tax_amount),
            remarks: Set(line.remarks),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|err| err.to_string())?;
        set_goods_line_taxes(&txn, copied.id, &tax_ids)
            .await
            .map_err(|err| err.to_string())?;
    }
    for line in services {
        let tax_ids = crate::logic::tax::load_service_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let copied = quotation_service_line::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            quotation_id: Set(saved.id),
            service_id: Set(line.service_id),
            rate: Set(line.rate),
            quantity: Set(line.quantity),
            variable_values: Set(line.variable_values),
            pre_tax_amount: Set(line.pre_tax_amount),
            remarks: Set(line.remarks),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|err| err.to_string())?;
        set_service_line_taxes(&txn, copied.id, &tax_ids)
            .await
            .map_err(|err| err.to_string())?;
    }
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(saved)
}
