//! Line editor form JSON and detail rows.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;

use crate::entities::{
    quotation_goods_line::{self, Entity as GoodsLineEntity},
    quotation_service_line::{self, Entity as ServiceLineEntity},
};
use crate::logic::tax::{load_goods_line_tax_ids, load_service_line_tax_ids};
use lariv_plugin_finance_accounts::scope::{CurrencyFormat, load_default_currency_format};
use lariv_plugin_finance_common::decimal;
use lariv_plugin_finance_invoices::logic::invoice_goods_line_editor::{
    InvoiceGoodsLineDisplayRow, default_lines_json as default_goods_json,
};
use lariv_plugin_finance_invoices::logic::invoice_service_line_editor::{
    InvoiceServiceLineDisplayRow, default_lines_json as default_service_json,
};
use lariv_plugin_finance_invoices::logic::optional_display;
use lariv_plugin_finance_invoices::logic::tax_calculations::invoice_line_amounts;
use lariv_plugin_finance_products::{
    entities::product::Entity as ProductEntity, pricing as product_pricing,
};
use lariv_plugin_finance_services::{
    entities::service::Entity as ServiceEntity, pricing as service_pricing,
};
use lariv_plugin_finance_taxes::scope::{load_taxes_by_ids, tax_label};

#[derive(Serialize)]
struct VariableRow {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    placeholder: String,
    value: String,
    unit: String,
}

fn variable_rows(schema_json: &str, has_formula: bool, stored: &str) -> Vec<VariableRow> {
    if !has_formula {
        return Vec::new();
    }
    let stored: serde_json::Value = serde_json::from_str(stored).unwrap_or(serde_json::json!({}));
    product_pricing::variable_input_specs(schema_json)
        .into_iter()
        .map(|spec| {
            let fallback = if spec.accepted_unit.is_empty() {
                match spec.ty.as_str() {
                    "length" => "mm",
                    "weight" => "kg",
                    _ => "",
                }
            } else {
                spec.accepted_unit.as_str()
            };
            let (value, unit) = stored
                .get(&spec.name)
                .map(|raw| stored_magnitude(raw, fallback))
                .unwrap_or_else(|| (String::new(), fallback.to_string()));
            VariableRow {
                name: spec.name,
                ty: spec.ty,
                placeholder: spec.placeholder,
                value,
                unit,
            }
        })
        .collect()
}

fn stored_magnitude(value: &serde_json::Value, default_unit: &str) -> (String, String) {
    if let Some(obj) = value.as_object() {
        let magnitude = obj
            .get("value")
            .map(|raw| match raw {
                serde_json::Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        let unit = obj
            .get("unit")
            .and_then(|unit| unit.as_str())
            .map(str::trim)
            .filter(|unit| !unit.is_empty())
            .unwrap_or(default_unit)
            .to_string();
        return (magnitude, unit);
    }
    let magnitude = match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    (magnitude, default_unit.to_string())
}

#[derive(Serialize)]
struct GoodsFormRow {
    product_id: i64,
    quantity: String,
    rate: String,
    product_label: String,
    fk_slot: String,
    tax_ids: Vec<i64>,
    has_formula: bool,
    variable_rows: Vec<VariableRow>,
    pre_tax: String,
    remarks: String,
}

#[derive(Serialize)]
struct ServiceFormRow {
    service_id: i64,
    quantity: String,
    rate: String,
    service_label: String,
    fk_slot: String,
    tax_ids: Vec<i64>,
    has_formula: bool,
    variable_rows: Vec<VariableRow>,
    pre_tax: String,
    remarks: String,
}

pub async fn goods_form_json(db: &DatabaseConnection, quotation_id: i64) -> String {
    let lines = GoodsLineEntity::find()
        .filter(quotation_goods_line::Column::QuotationId.eq(quotation_id))
        .order_by_asc(quotation_goods_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    if lines.is_empty() {
        return default_goods_json();
    }
    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        let product = ProductEntity::find_by_id(line.product_id)
            .one(db)
            .await
            .ok()
            .flatten();
        let has_formula = product
            .as_ref()
            .is_some_and(|row| product_pricing::has_variable_pricing(&row.variables));
        let variable_rows = product
            .as_ref()
            .map(|row| variable_rows(&row.variables, has_formula, &line.variable_values))
            .unwrap_or_default();
        let product_label = product.map(|row| row.name).unwrap_or_default();
        let tax_ids = load_goods_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        rows.push(GoodsFormRow {
            product_id: line.product_id,
            quantity: decimal::decimal_display(line.quantity),
            rate: decimal::decimal_display(line.rate),
            product_label,
            fk_slot: format!("InvoiceLineProduct_{quotation_id}_{}", line.id),
            tax_ids,
            has_formula,
            variable_rows,
            pre_tax: decimal::decimal_display(line.pre_tax_amount),
            remarks: line.remarks.unwrap_or_default(),
        });
    }
    serde_json::to_string(&rows).unwrap_or_else(|_| default_goods_json())
}

pub async fn service_form_json(db: &DatabaseConnection, quotation_id: i64) -> String {
    let lines = ServiceLineEntity::find()
        .filter(quotation_service_line::Column::QuotationId.eq(quotation_id))
        .order_by_asc(quotation_service_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    if lines.is_empty() {
        return default_service_json();
    }
    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        let service = ServiceEntity::find_by_id(line.service_id)
            .one(db)
            .await
            .ok()
            .flatten();
        let has_formula = service
            .as_ref()
            .is_some_and(|row| service_pricing::has_variable_pricing(&row.variables));
        let variable_rows = service
            .as_ref()
            .map(|row| variable_rows(&row.variables, has_formula, &line.variable_values))
            .unwrap_or_default();
        let service_label = service.map(|row| row.name).unwrap_or_default();
        let tax_ids = load_service_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        rows.push(ServiceFormRow {
            service_id: line.service_id,
            quantity: decimal::decimal_display(line.quantity),
            rate: decimal::decimal_display(line.rate),
            service_label,
            fk_slot: format!("InvoiceLineService_{quotation_id}_{}", line.id),
            tax_ids,
            has_formula,
            variable_rows,
            pre_tax: decimal::decimal_display(line.pre_tax_amount),
            remarks: line.remarks.unwrap_or_default(),
        });
    }
    serde_json::to_string(&rows).unwrap_or_else(|_| default_service_json())
}

pub async fn goods_display_rows(
    db: &DatabaseConnection,
    quotation_id: i64,
) -> Vec<InvoiceGoodsLineDisplayRow> {
    let lines = GoodsLineEntity::find()
        .filter(quotation_goods_line::Column::QuotationId.eq(quotation_id))
        .order_by_asc(quotation_goods_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let currency = load_default_currency_format(db).await;
    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        let product = ProductEntity::find_by_id(line.product_id)
            .one(db)
            .await
            .ok()
            .flatten();
        let name = product
            .as_ref()
            .map(|row| row.name.clone())
            .unwrap_or_else(|| format!("#{}", line.product_id));
        let inputs = product
            .as_ref()
            .filter(|row| product_pricing::has_variable_pricing(&row.variables))
            .and_then(|row| {
                product_pricing::format_variable_inputs(&row.variables, &line.variable_values)
            })
            .unwrap_or_else(|| {
                product_pricing::inputs_display(&line.variable_values, line.quantity, line.rate)
            });
        let tax_ids = load_goods_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        rows.push(
            line_amounts(
                &currency,
                name,
                inputs,
                &line.remarks,
                line.quantity,
                line.rate,
                line.pre_tax_amount,
                &tax_ids,
                db,
            )
            .await,
        );
    }
    rows
}

pub async fn service_display_rows(
    db: &DatabaseConnection,
    quotation_id: i64,
) -> Vec<InvoiceServiceLineDisplayRow> {
    let lines = ServiceLineEntity::find()
        .filter(quotation_service_line::Column::QuotationId.eq(quotation_id))
        .order_by_asc(quotation_service_line::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let currency = load_default_currency_format(db).await;
    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        let service = ServiceEntity::find_by_id(line.service_id)
            .one(db)
            .await
            .ok()
            .flatten();
        let name = service
            .as_ref()
            .map(|row| row.name.clone())
            .unwrap_or_else(|| format!("#{}", line.service_id));
        let inputs = service
            .as_ref()
            .filter(|row| service_pricing::has_variable_pricing(&row.variables))
            .and_then(|row| {
                service_pricing::format_variable_inputs(&row.variables, &line.variable_values)
            })
            .unwrap_or_else(|| {
                service_pricing::inputs_display(&line.variable_values, line.quantity, line.rate)
            });
        let tax_ids = load_service_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let amounts = line_amounts(
            &currency,
            name,
            inputs,
            &line.remarks,
            line.quantity,
            line.rate,
            line.pre_tax_amount,
            &tax_ids,
            db,
        )
        .await;
        rows.push(InvoiceServiceLineDisplayRow {
            service: amounts.product,
            remarks: amounts.remarks,
            inputs: amounts.inputs,
            quantity: amounts.quantity,
            rate: amounts.rate,
            line_taxes: amounts.line_taxes,
            untaxed_amount: amounts.untaxed_amount,
            levied_tax_amount: amounts.levied_tax_amount,
            withholding_amount: amounts.withholding_amount,
            line_total: amounts.line_total,
        });
    }
    rows
}

#[expect(
    clippy::too_many_arguments,
    reason = "line display fields stay explicit"
)]
async fn line_amounts(
    currency: &CurrencyFormat,
    name: String,
    inputs: String,
    remarks: &Option<String>,
    quantity: Decimal,
    rate: Decimal,
    pre_tax: Decimal,
    tax_ids: &[i64],
    db: &DatabaseConnection,
) -> InvoiceGoodsLineDisplayRow {
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let line_taxes = if taxes.is_empty() {
        "—".to_string()
    } else {
        taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ")
    };
    let (untaxed, levied, withholding, net) = invoice_line_amounts(pre_tax, &taxes);
    InvoiceGoodsLineDisplayRow {
        product: name,
        remarks: optional_display(remarks),
        inputs,
        quantity: decimal::decimal_display(quantity),
        rate: currency.display(rate),
        line_taxes,
        untaxed_amount: currency.display(untaxed),
        levied_tax_amount: currency.display(levied),
        withholding_amount: decimal::decimal_display_withholding(
            withholding,
            currency.minor_unit,
            &currency.symbol,
        ),
        line_total: currency.display(net),
    }
}

pub async fn quotation_grand_total(db: &DatabaseConnection, quotation_id: i64) -> Decimal {
    let currency_unused = Decimal::ZERO;
    let mut total = currency_unused;
    let goods = GoodsLineEntity::find()
        .filter(quotation_goods_line::Column::QuotationId.eq(quotation_id))
        .all(db)
        .await
        .unwrap_or_default();
    for line in goods {
        let tax_ids = load_goods_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(db, &tax_ids).await.unwrap_or_default();
        let (_, _, _, net) = invoice_line_amounts(line.pre_tax_amount, &taxes);
        total += net;
    }
    let services = ServiceLineEntity::find()
        .filter(quotation_service_line::Column::QuotationId.eq(quotation_id))
        .all(db)
        .await
        .unwrap_or_default();
    for line in services {
        let tax_ids = load_service_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        let taxes = load_taxes_by_ids(db, &tax_ids).await.unwrap_or_default();
        let (_, _, _, net) = invoice_line_amounts(line.pre_tax_amount, &taxes);
        total += net;
    }
    total
}
