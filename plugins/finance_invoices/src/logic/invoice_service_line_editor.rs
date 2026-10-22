//! Invoice line editor preview JSON and form defaults.

use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use serde::Serialize;

use crate::logic::bill_to::{BillTo, PartyLabels};
use lariv_plugin_finance_accounts::scope::{
    CurrencyFormat, load_default_currency_format, load_journal_currency_format,
};
use lariv_plugin_finance_common::decimal;
use lariv_plugin_finance_services::{
    entities::service::Entity as ServiceEntity, preferences::load_service_tax_ids, pricing,
};
use lariv_plugin_finance_taxes::{
    entities::tax::TaxKind,
    scope::{load_all_taxes, load_taxes_by_ids, tax_label},
};

use crate::entities::{
    cancelled_invoice::Entity as CancelledInvoiceEntity,
    draft_invoice_service_line::{self, Entity as DraftInvoiceServiceLineEntity},
    posted_invoice::Entity as PostedInvoiceEntity,
    posted_invoice_service_line::{self, Entity as PostedInvoiceServiceLineEntity},
};
use crate::logic::preferences::load_invoice_preferences;
use crate::logic::tax_assoc::{
    load_cancelled_service_line_tax_ids, load_draft_service_line_tax_ids,
    load_posted_service_line_tax_ids,
};

pub fn default_lines_json() -> String {
    r#"[{"service_id":0,"quantity":"1","rate":"","service_label":"","fk_slot":"line-slot-0","tax_ids":[],"has_formula":false,"variable_rows":[],"pre_tax":"","remarks":""}]"#
        .to_string()
}

#[derive(Serialize)]
struct InvoiceLineServiceOpt {
    id: i64,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sales_price: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tax_ids: Vec<i64>,
    has_formula: bool,
    variables: Vec<pricing::VariableInputSpec>,
    remarks: String,
}

#[derive(Serialize)]
struct InvoiceLineTaxMeta {
    id: i64,
    name: String,
    tax_kind: String,
}

#[derive(Serialize)]
struct InvoiceLineEditorPreview {
    services: Vec<InvoiceLineServiceOpt>,
    tax_pct_by_id: std::collections::HashMap<String, String>,
    tax_kind_by_id: std::collections::HashMap<String, String>,
    all_taxes: Vec<InvoiceLineTaxMeta>,
}

pub async fn invoice_service_line_editor_preview_json(db: &DatabaseConnection) -> String {
    let products = ServiceEntity::find().all(db).await.unwrap_or_default();

    let mut product_opts = Vec::with_capacity(products.len());
    for p in products {
        let tax_ids = load_service_tax_ids(db, p.id).await;
        let has_formula = pricing::has_variable_pricing(&p.variables);
        let sales_price = if has_formula {
            None
        } else {
            pricing::unit_amount(&p.sales_price_formula)
                .ok()
                .filter(|amount| !decimal::dec_is_zero(*amount))
                .map(|amount| decimal::decimal_display(amount))
        };
        product_opts.push(InvoiceLineServiceOpt {
            id: p.id,
            name: p.name.clone(),
            sales_price,
            tax_ids,
            has_formula,
            variables: pricing::variable_input_specs(&p.variables),
            remarks: p.remarks.unwrap_or_default(),
        });
    }

    let taxes = load_all_taxes(db).await.unwrap_or_default();
    let mut tax_pct_by_id = std::collections::HashMap::new();
    let mut tax_kind_by_id = std::collections::HashMap::new();
    let mut all_taxes = Vec::with_capacity(taxes.len());
    for t in taxes {
        let id = t.id.to_string();
        tax_pct_by_id.insert(id.clone(), decimal::decimal_display(t.percentage));
        tax_kind_by_id.insert(
            id.clone(),
            match t.tax_type {
                TaxKind::Withholding => "withholding",
                TaxKind::Levied => "levied",
            }
            .to_string(),
        );
        all_taxes.push(InvoiceLineTaxMeta {
            id: t.id,
            name: tax_label(&t),
            tax_kind: tax_kind_by_id[&id].clone(),
        });
    }

    let preview = InvoiceLineEditorPreview {
        services: product_opts,
        tax_pct_by_id,
        tax_kind_by_id,
        all_taxes,
    };
    serde_json::to_string(&preview).unwrap_or_else(|_| "{}".to_string())
}

#[derive(Serialize)]
struct LineVariableRow {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    placeholder: String,
    value: String,
    /// Unit entered for length (`mm`, `cm`, …) or weight (`kg`). Empty otherwise.
    unit: String,
}

#[derive(Serialize)]
struct DraftLineFormRow {
    service_id: i64,
    quantity: String,
    rate: String,
    service_label: String,
    fk_slot: String,
    tax_ids: Vec<i64>,
    has_formula: bool,
    variable_rows: Vec<LineVariableRow>,
    pre_tax: String,
    remarks: String,
}

fn default_variable_unit(ty: &str) -> &'static str {
    match ty {
        "length" => "mm",
        "weight" => "kg",
        _ => "",
    }
}

/// Magnitude and unit from a stored variable. Objects are `{value, unit}`.
/// A bare number is the magnitude in the type's default unit.
fn stored_magnitude(value: &serde_json::Value, default_unit: &str) -> (String, String) {
    if let Some(obj) = value.as_object() {
        let magnitude = obj
            .get("value")
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
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
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    (magnitude, default_unit.to_string())
}

fn variable_rows_for_line(
    schema_json: &str,
    has_formula: bool,
    stored: &str,
) -> Vec<LineVariableRow> {
    if !has_formula {
        return Vec::new();
    }
    let stored: serde_json::Value = serde_json::from_str(stored).unwrap_or(serde_json::json!({}));
    pricing::variable_input_specs(schema_json)
        .into_iter()
        .map(|spec| {
            let fallback = if spec.accepted_unit.is_empty() {
                default_variable_unit(&spec.ty).to_string()
            } else {
                spec.accepted_unit.clone()
            };
            let (value, unit) = stored
                .get(&spec.name)
                .map(|v| stored_magnitude(v, &fallback))
                .unwrap_or_else(|| (String::new(), fallback));
            LineVariableRow {
                name: spec.name,
                ty: spec.ty,
                placeholder: spec.placeholder,
                value,
                unit,
            }
        })
        .collect()
}

pub async fn draft_lines_form_json(db: &DatabaseConnection, draft_id: i64) -> String {
    let lines = DraftInvoiceServiceLineEntity::find()
        .filter(draft_invoice_service_line::Column::DraftInvoiceId.eq(draft_id))
        .all(db)
        .await
        .unwrap_or_default();

    if lines.is_empty() {
        return default_lines_json();
    }

    let mut rows = Vec::with_capacity(lines.len());
    for ln in lines {
        let product = lariv_core::web::opt_or_log(
            ServiceEntity::find_by_id(ln.service_id).one(db).await,
            "find by id",
        );
        let has_formula = product
            .as_ref()
            .is_some_and(|p| pricing::has_variable_pricing(&p.variables));
        let variable_rows = product
            .as_ref()
            .map(|p| variable_rows_for_line(&p.variables, has_formula, &ln.variable_values))
            .unwrap_or_default();
        let service_label = product.map(|p| p.name).unwrap_or_default();
        let tax_ids = load_draft_service_line_tax_ids(db, ln.id)
            .await
            .unwrap_or_default();
        rows.push(DraftLineFormRow {
            service_id: ln.service_id,
            quantity: decimal::decimal_display(ln.quantity),
            rate: decimal::decimal_display(ln.rate),
            service_label,
            fk_slot: format!("InvoiceLineService_{draft_id}_{}", ln.id),
            tax_ids,
            has_formula,
            variable_rows,
            pre_tax: decimal::decimal_display(ln.pre_tax_amount),
            remarks: ln.remarks.unwrap_or_default(),
        });
    }

    serde_json::to_string(&rows).unwrap_or_else(|_| default_lines_json())
}

#[derive(Clone, Debug)]
pub struct InvoiceServiceLineDisplayRow {
    pub service: String,
    pub remarks: String,
    pub inputs: String,
    pub quantity: String,
    pub rate: String,
    pub line_taxes: String,
    pub untaxed_amount: String,
    pub levied_tax_amount: String,
    pub withholding_amount: String,
    pub line_total: String,
}

async fn currency_for_journal_or_default(
    db: &DatabaseConnection,
    journal_id: Option<i64>,
) -> CurrencyFormat {
    match journal_id.filter(|&id| id > 0) {
        Some(id) => load_journal_currency_format(db, id).await,
        None => load_default_currency_format(db).await,
    }
}

pub async fn invoice_customer_name(db: &DatabaseConnection, party: BillTo) -> String {
    PartyLabels::one(db, party).await
}

pub async fn invoice_header_tax_labels(db: &DatabaseConnection, tax_ids: &[i64]) -> String {
    if tax_ids.is_empty() {
        return "—".to_string();
    }
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    if taxes.is_empty() {
        "—".to_string()
    } else {
        taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ")
    }
}

async fn build_line_display_row(
    db: &DatabaseConnection,
    service_id: i64,
    quantity: Decimal,
    rate: Decimal,
    variable_values: &str,
    pre_tax: Decimal,
    remarks: &Option<String>,
    tax_ids: &[i64],
    currency: &CurrencyFormat,
) -> InvoiceServiceLineDisplayRow {
    let product = lariv_core::web::opt_or_log(
        ServiceEntity::find_by_id(service_id).one(db).await,
        "find by id",
    );
    let product_name = product
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| format!("#{service_id}"));
    let inputs = product
        .as_ref()
        .filter(|p| pricing::has_variable_pricing(&p.variables))
        .and_then(|p| pricing::format_variable_inputs(&p.variables, variable_values))
        .unwrap_or_else(|| pricing::inputs_display(variable_values, quantity, rate));
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let line_taxes = if taxes.is_empty() {
        "—".to_string()
    } else {
        taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ")
    };
    let (untaxed, levied, withholding, net) =
        crate::logic::tax_calculations::invoice_line_amounts(pre_tax, &taxes);
    InvoiceServiceLineDisplayRow {
        service: product_name,
        remarks: crate::logic::draft::optional_display(remarks),
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

pub async fn draft_invoice_service_line_display_rows(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Vec<InvoiceServiceLineDisplayRow> {
    let lines = DraftInvoiceServiceLineEntity::find()
        .filter(draft_invoice_service_line::Column::DraftInvoiceId.eq(draft_id))
        .all(db)
        .await
        .unwrap_or_default();
    let prefs = load_invoice_preferences(db).await;
    let currency = currency_for_journal_or_default(db, prefs.journal_id).await;

    let mut rows = Vec::with_capacity(lines.len());
    for ln in lines {
        let tax_ids = load_draft_service_line_tax_ids(db, ln.id)
            .await
            .unwrap_or_default();
        rows.push(
            build_line_display_row(
                db,
                ln.service_id,
                ln.quantity,
                ln.rate,
                &ln.variable_values,
                ln.pre_tax_amount,
                &ln.remarks,
                &tax_ids,
                &currency,
            )
            .await,
        );
    }
    rows
}

pub async fn posted_invoice_service_line_display_rows(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Vec<InvoiceServiceLineDisplayRow> {
    let lines = PostedInvoiceServiceLineEntity::find()
        .filter(posted_invoice_service_line::Column::PostedInvoiceId.eq(posted_id))
        .all(db)
        .await
        .unwrap_or_default();
    let journal_id = lariv_core::web::opt_or_log(
        PostedInvoiceEntity::find_by_id(posted_id).one(db).await,
        "find by id",
    )
    .map(|p| p.journal_id);
    let currency = currency_for_journal_or_default(db, journal_id).await;

    let mut rows = Vec::with_capacity(lines.len());
    for ln in lines {
        let tax_ids = load_posted_service_line_tax_ids(db, ln.id)
            .await
            .unwrap_or_default();
        rows.push(
            build_line_display_row(
                db,
                ln.service_id,
                ln.quantity,
                ln.rate,
                &ln.variable_values,
                ln.pre_tax_amount,
                &ln.remarks,
                &tax_ids,
                &currency,
            )
            .await,
        );
    }
    rows
}

struct CancelledLineRow {
    id: i64,
    service_id: i64,
    rate: Decimal,
    quantity: Decimal,
    variable_values: String,
    pre_tax_amount: Decimal,
    remarks: Option<String>,
}

async fn load_cancelled_invoice_service_lines(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Vec<CancelledLineRow> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT id, service_id, rate, quantity, variable_values, pre_tax_amount, remarks \
             FROM cancelled_invoice_service_lines \
             WHERE cancelled_invoice_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .unwrap_or_default();

    rows.into_iter()
        .filter_map(|r| {
            Some(CancelledLineRow {
                id: r.try_get("", "id").ok()?,
                service_id: r.try_get("", "service_id").ok()?,
                rate: r.try_get("", "rate").ok()?,
                quantity: r.try_get("", "quantity").ok()?,
                variable_values: r.try_get("", "variable_values").ok()?,
                pre_tax_amount: r.try_get("", "pre_tax_amount").ok()?,
                remarks: r.try_get("", "remarks").ok()?,
            })
        })
        .collect()
}

pub async fn cancelled_invoice_service_line_display_rows(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Vec<InvoiceServiceLineDisplayRow> {
    let lines = load_cancelled_invoice_service_lines(db, cancelled_id).await;
    let journal_id = lariv_core::web::opt_or_log(
        CancelledInvoiceEntity::find_by_id(cancelled_id)
            .one(db)
            .await,
        "find by id",
    )
    .map(|c| c.journal_id);
    let currency = currency_for_journal_or_default(db, journal_id).await;
    let mut rows = Vec::with_capacity(lines.len());
    for ln in lines {
        let tax_ids = load_cancelled_service_line_tax_ids(db, ln.id)
            .await
            .unwrap_or_default();
        rows.push(
            build_line_display_row(
                db,
                ln.service_id,
                ln.quantity,
                ln.rate,
                &ln.variable_values,
                ln.pre_tax_amount,
                &ln.remarks,
                &tax_ids,
                &currency,
            )
            .await,
        );
    }
    rows
}
