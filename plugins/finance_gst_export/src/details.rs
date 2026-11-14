//! Zero-or-one GST return details for each invoice state.

use chrono::NaiveDate;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GstDetails {
    pub gst_section: String,
    pub gst_inv_typ: String,
    pub reverse_charge: bool,
    pub etin: Option<String>,
    pub supplier_gstin: Option<String>,
    pub exp_typ: Option<String>,
    pub shipping_bill_number: Option<String>,
    pub shipping_bill_date: Option<NaiveDate>,
    pub port_code: Option<String>,
    pub amendment_of_number: Option<String>,
    pub amendment_of_date: Option<NaiveDate>,
}

impl Default for GstDetails {
    fn default() -> Self {
        Self {
            gst_section: "regular".to_string(),
            gst_inv_typ: "R".to_string(),
            reverse_charge: false,
            etin: None,
            supplier_gstin: None,
            exp_typ: None,
            shipping_bill_number: None,
            shipping_bill_date: None,
            port_code: None,
            amendment_of_number: None,
            amendment_of_date: None,
        }
    }
}

impl GstDetails {
    pub fn summary(&self) -> String {
        let reverse = if self.reverse_charge { "yes" } else { "no" };
        let mut parts = vec![
            format!("Section {}", self.gst_section),
            format!("type {}", self.gst_inv_typ),
            format!("reverse charge {reverse}"),
        ];
        if let Some(etin) = &self.etin {
            parts.push(format!("e-commerce GSTIN {etin}"));
        }
        if let Some(supplier) = &self.supplier_gstin {
            parts.push(format!("supplier GSTIN {supplier}"));
        }
        if let Some(exp) = &self.exp_typ {
            parts.push(format!("export {exp}"));
        }
        if let Some(bill) = &self.shipping_bill_number {
            parts.push(format!("shipping bill {bill}"));
        }
        if let Some(date) = self.shipping_bill_date {
            parts.push(format!("shipping bill date {date}"));
        }
        if let Some(port) = &self.port_code {
            parts.push(format!("port {port}"));
        }
        if let Some(number) = &self.amendment_of_number {
            parts.push(format!("amends {number}"));
        }
        if let Some(date) = self.amendment_of_date {
            parts.push(format!("original date {date}"));
        }
        parts.join("; ")
    }

    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Clone, Copy)]
pub enum GstTable {
    Draft,
    Posted,
    Cancelled,
    Quotation,
}

impl GstTable {
    fn table(self) -> &'static str {
        match self {
            Self::Draft => "draft_invoice_gst_details",
            Self::Posted => "posted_invoice_gst_details",
            Self::Cancelled => "cancelled_invoice_gst_details",
            Self::Quotation => "quotation_invoice_gst_details",
        }
    }

    fn id_col(self) -> &'static str {
        match self {
            Self::Draft => "draft_invoice_id",
            Self::Posted => "posted_invoice_id",
            Self::Cancelled => "cancelled_invoice_id",
            Self::Quotation => "quotation_invoice_id",
        }
    }
}

pub fn normalize_section(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok("regular".to_string());
    }
    match value {
        "regular" | "export" | "ecom_b2b" | "ecom_b2c" | "ecom_urp2b" | "ecom_urp2c"
        | "supeco_clttx" | "supeco_paytx" => Ok(value.to_string()),
        _ => Err(format!("unknown GST section {value}")),
    }
}

pub fn normalize_inv_typ(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok("R".to_string());
    }
    match value {
        "R" | "SEWP" | "SEWOP" | "DE" | "CBW" => Ok(value.to_string()),
        _ => Err(format!("unknown GST invoice type {value}")),
    }
}

pub fn normalize_exp_typ(raw: &str) -> Result<Option<String>, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None);
    }
    match value {
        "WPAY" | "WOPAY" => Ok(Some(value.to_string())),
        _ => Err(format!("unknown export type {value}")),
    }
}

pub fn normalize_supply(raw: &str) -> String {
    match raw.trim() {
        "nil" | "exempt" | "nongst" => raw.trim().to_string(),
        _ => "taxable".to_string(),
    }
}

pub fn parse_reverse_charge(raw: &str) -> bool {
    matches!(raw.trim(), "true" | "1" | "yes" | "Y")
}

pub async fn load_gst<C: ConnectionTrait>(
    db: &C,
    table: GstTable,
    id: i64,
) -> Result<GstDetails, String> {
    let sql = format!(
        "SELECT gst_section, gst_inv_typ, reverse_charge, etin, supplier_gstin, exp_typ, \
         shipping_bill_number, shipping_bill_date, port_code, amendment_of_number, \
         amendment_of_date FROM {} WHERE {} = $1",
        table.table(),
        table.id_col()
    );
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    let Some(row) = row else {
        return Ok(GstDetails::default());
    };
    Ok(GstDetails {
        gst_section: row
            .try_get("", "gst_section")
            .unwrap_or_else(|_| "regular".to_string()),
        gst_inv_typ: row
            .try_get("", "gst_inv_typ")
            .unwrap_or_else(|_| "R".to_string()),
        reverse_charge: row.try_get("", "reverse_charge").unwrap_or(false),
        etin: row.try_get("", "etin").unwrap_or(None),
        supplier_gstin: row.try_get("", "supplier_gstin").unwrap_or(None),
        exp_typ: row.try_get("", "exp_typ").unwrap_or(None),
        shipping_bill_number: row.try_get("", "shipping_bill_number").unwrap_or(None),
        shipping_bill_date: row.try_get("", "shipping_bill_date").unwrap_or(None),
        port_code: row.try_get("", "port_code").unwrap_or(None),
        amendment_of_number: row.try_get("", "amendment_of_number").unwrap_or(None),
        amendment_of_date: row.try_get("", "amendment_of_date").unwrap_or(None),
    })
}

pub async fn upsert_gst<C: ConnectionTrait>(
    db: &C,
    table: GstTable,
    id: i64,
    details: &GstDetails,
) -> Result<(), String> {
    let id_col = table.id_col();
    let sql = format!(
        "INSERT INTO {table} ({id_col}, gst_section, gst_inv_typ, reverse_charge, etin, \
         supplier_gstin, exp_typ, shipping_bill_number, shipping_bill_date, port_code, \
         amendment_of_number, amendment_of_date) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         ON CONFLICT ({id_col}) DO UPDATE SET \
         gst_section = EXCLUDED.gst_section, \
         gst_inv_typ = EXCLUDED.gst_inv_typ, \
         reverse_charge = EXCLUDED.reverse_charge, \
         etin = EXCLUDED.etin, \
         supplier_gstin = EXCLUDED.supplier_gstin, \
         exp_typ = EXCLUDED.exp_typ, \
         shipping_bill_number = EXCLUDED.shipping_bill_number, \
         shipping_bill_date = EXCLUDED.shipping_bill_date, \
         port_code = EXCLUDED.port_code, \
         amendment_of_number = EXCLUDED.amendment_of_number, \
         amendment_of_date = EXCLUDED.amendment_of_date",
        table = table.table(),
    );
    let values: Vec<Value> = vec![
        id.into(),
        details.gst_section.clone().into(),
        details.gst_inv_typ.clone().into(),
        details.reverse_charge.into(),
        details.etin.clone().into(),
        details.supplier_gstin.clone().into(),
        details.exp_typ.clone().into(),
        details.shipping_bill_number.clone().into(),
        details.shipping_bill_date.into(),
        details.port_code.clone().into(),
        details.amendment_of_number.clone().into(),
        details.amendment_of_date.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        values,
    ))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn copy_gst<C: ConnectionTrait>(
    db: &C,
    from: GstTable,
    from_id: i64,
    to: GstTable,
    to_id: i64,
) -> Result<(), String> {
    let loaded = load_gst(db, from, from_id).await?;
    if loaded.is_default() {
        let sql = format!(
            "SELECT 1 AS present FROM {} WHERE {} = $1",
            from.table(),
            from.id_col()
        );
        let row = db
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                sql,
                [from_id.into()],
            ))
            .await
            .map_err(|e| e.to_string())?;
        if row.is_none() {
            return Ok(());
        }
    }
    upsert_gst(db, to, to_id, &loaded).await
}

pub struct GstFormValues {
    pub gst_section: String,
    pub gst_inv_typ: String,
    pub reverse_charge: String,
    pub etin: String,
    pub supplier_gstin: String,
    pub exp_typ: String,
    pub shipping_bill_number: String,
    pub shipping_bill_date: String,
    pub port_code: String,
    pub amendment_of_number: String,
    pub amendment_of_date: String,
}

impl GstFormValues {
    pub fn blank() -> Self {
        Self::from_details(&GstDetails::default())
    }

    pub fn from_details(details: &GstDetails) -> Self {
        Self {
            gst_section: details.gst_section.clone(),
            gst_inv_typ: details.gst_inv_typ.clone(),
            reverse_charge: if details.reverse_charge {
                "true".to_string()
            } else {
                "false".to_string()
            },
            etin: details.etin.clone().unwrap_or_default(),
            supplier_gstin: details.supplier_gstin.clone().unwrap_or_default(),
            exp_typ: details.exp_typ.clone().unwrap_or_default(),
            shipping_bill_number: details.shipping_bill_number.clone().unwrap_or_default(),
            shipping_bill_date: details
                .shipping_bill_date
                .map(|date| date.format("%d/%m/%Y").to_string())
                .unwrap_or_default(),
            port_code: details.port_code.clone().unwrap_or_default(),
            amendment_of_number: details.amendment_of_number.clone().unwrap_or_default(),
            amendment_of_date: details
                .amendment_of_date
                .map(|date| date.format("%d/%m/%Y").to_string())
                .unwrap_or_default(),
        }
    }

    pub fn any_filled(&self) -> bool {
        !self.gst_section.trim().is_empty()
            || !self.gst_inv_typ.trim().is_empty()
            || !self.reverse_charge.trim().is_empty()
            || !self.etin.trim().is_empty()
            || !self.supplier_gstin.trim().is_empty()
            || !self.exp_typ.trim().is_empty()
            || !self.shipping_bill_number.trim().is_empty()
            || !self.shipping_bill_date.trim().is_empty()
            || !self.port_code.trim().is_empty()
            || !self.amendment_of_number.trim().is_empty()
            || !self.amendment_of_date.trim().is_empty()
    }
}

pub fn name_is_gst_component(name: &str) -> bool {
    let upper = name.trim().to_ascii_uppercase();
    upper.starts_with("IGST")
        || upper.starts_with("CGST")
        || upper.starts_with("SGST")
        || upper.starts_with("UTGST")
        || upper.starts_with("CESS")
}

pub fn supply_for_line(raw: &str, has_gst_tax: bool) -> String {
    if has_gst_tax {
        "taxable".to_string()
    } else {
        normalize_supply(raw)
    }
}

pub fn gst_from_form(values: &GstFormValues) -> Result<GstDetails, String> {
    let mut details = GstDetails {
        gst_section: normalize_section(&values.gst_section)?,
        gst_inv_typ: normalize_inv_typ(&values.gst_inv_typ)?,
        reverse_charge: parse_reverse_charge(&values.reverse_charge),
        etin: blank_opt(&values.etin),
        supplier_gstin: blank_opt(&values.supplier_gstin),
        exp_typ: normalize_exp_typ(&values.exp_typ)?,
        shipping_bill_number: blank_opt(&values.shipping_bill_number),
        shipping_bill_date: parse_optional_date(&values.shipping_bill_date)?,
        port_code: blank_opt(&values.port_code),
        amendment_of_number: blank_opt(&values.amendment_of_number),
        amendment_of_date: parse_optional_date(&values.amendment_of_date)?,
    };
    apply_section(&mut details)?;
    Ok(details)
}

fn apply_section(details: &mut GstDetails) -> Result<(), String> {
    match details.gst_section.as_str() {
        "export" => {
            if details.exp_typ.is_none() {
                return Err("an export needs an export type".into());
            }
            details.gst_inv_typ = "R".to_string();
            details.reverse_charge = false;
            details.etin = None;
            details.supplier_gstin = None;
        }
        "ecom_b2b" | "ecom_urp2b" => {
            if details.supplier_gstin.is_none() {
                return Err("an e-commerce invoice needs the supplier GSTIN".into());
            }
            details.reverse_charge = false;
            details.etin = None;
            clear_export(details);
        }
        "ecom_b2c" | "ecom_urp2c" => {
            if details.supplier_gstin.is_none() {
                return Err("an e-commerce invoice needs the supplier GSTIN".into());
            }
            details.gst_inv_typ = "R".to_string();
            details.reverse_charge = false;
            details.etin = None;
            clear_export(details);
        }
        "supeco_clttx" | "supeco_paytx" => {
            if details.etin.is_none() {
                return Err("ECO supplies need the e-commerce operator GSTIN".into());
            }
            details.gst_inv_typ = "R".to_string();
            details.reverse_charge = false;
            details.supplier_gstin = None;
            clear_export(details);
        }
        _ => {
            details.etin = None;
            details.supplier_gstin = None;
            clear_export(details);
            if details.gst_inv_typ == "DE" {
                details.reverse_charge = false;
            }
        }
    }
    Ok(())
}

fn clear_export(details: &mut GstDetails) {
    details.exp_typ = None;
    details.shipping_bill_number = None;
    details.shipping_bill_date = None;
    details.port_code = None;
}

pub async fn copy_line_supply<C: ConnectionTrait>(
    db: &C,
    from_lines: &str,
    from_parent_col: &str,
    from_parent: i64,
    to_lines: &str,
    to_parent_col: &str,
    to_parent: i64,
) -> Result<(), String> {
    let from_rows = line_supplies(db, from_lines, from_parent_col, from_parent).await?;
    let to_ids = line_ids(db, to_lines, to_parent_col, to_parent).await?;
    if from_rows.len() != to_ids.len() {
        return Err("GST line supply copy found a different number of lines".into());
    }
    for (supply, id) in from_rows.into_iter().zip(to_ids) {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("UPDATE {to_lines} SET gst_supply = $1 WHERE id = $2"),
            [supply.into(), id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn line_supplies<C: ConnectionTrait>(
    db: &C,
    table: &str,
    parent_col: &str,
    parent: i64,
) -> Result<Vec<String>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT gst_supply FROM {table} WHERE {parent_col} = $1 ORDER BY id"),
            [parent.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    rows.iter()
        .map(|row| {
            row.try_get::<String>("", "gst_supply")
                .map_err(|e| e.to_string())
        })
        .collect()
}

async fn line_ids<C: ConnectionTrait>(
    db: &C,
    table: &str,
    parent_col: &str,
    parent: i64,
) -> Result<Vec<i64>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT id FROM {table} WHERE {parent_col} = $1 ORDER BY id"),
            [parent.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    rows.iter()
        .map(|row| row.try_get::<i64>("", "id").map_err(|e| e.to_string()))
        .collect()
}

pub fn line_table(table: GstTable) -> (&'static str, &'static str) {
    match table {
        GstTable::Draft => ("draft_invoice_lines", "draft_invoice_id"),
        GstTable::Posted => ("posted_invoice_lines", "posted_invoice_id"),
        GstTable::Cancelled => ("cancelled_invoice_lines", "cancelled_invoice_id"),
        GstTable::Quotation => ("quotation_invoice_lines", "quotation_invoice_id"),
    }
}

fn parse_optional_date(raw: &str) -> Result<Option<NaiveDate>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    lariv_core::datetime::parse_date(raw)
        .map(Some)
        .ok_or_else(|| format!("invalid date {raw}"))
}

fn blank_opt(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
