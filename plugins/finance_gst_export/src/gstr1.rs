//! GSTR-1 JSON for one return month.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseBackend, QueryResult, Statement, Value as DbValue};
use serde_json::{Map, Value, json};

const VERSION: &str = "GST3.2.4";
const B2CL_LIMIT: i64 = 100_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Component {
    Igst,
    Cgst,
    Sgst,
    Cess,
}

struct Span {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    fp: String,
}

pub async fn build<C: ConnectionTrait>(
    db: &C,
    year: i32,
    month: u32,
    gt: Option<Decimal>,
    cur_gt: Option<Decimal>,
) -> Result<Vec<u8>, String> {
    let span = span(year, month)?;
    let gstin = seller_gstin(db).await?;
    let seller_state = state_code(&gstin)
        .ok_or_else(|| format!("company GSTIN {gstin} must start with a two-digit state code"))?;
    let components = load_components(db).await?;
    let invoices = load_invoices(db, &span).await?;
    let lines = load_lines(db, &span).await?;
    let line_taxes = load_taxes(
        db,
        &span,
        "posted_invoice_line_taxes",
        "posted_invoice_line_id",
    )
    .await?;
    let header_taxes = load_taxes(db, &span, "posted_invoice_taxes", "posted_invoice_id").await?;
    let notes = load_notes(db, &span).await?;
    let note_lines = load_note_lines(db, &span).await?;
    let note_taxes = load_note_taxes(db, &span).await?;
    let advances = load_advances(db, &span).await?;
    let adjustments = load_adjustments(db, &span).await?;
    let advance_taxes = load_advance_taxes(db).await?;

    let mut errors = Vec::new();
    let mut doc = Map::new();
    doc.insert("gstin".into(), json!(gstin));
    doc.insert("fp".into(), json!(span.fp));
    let (previous_fy, current_fy) = turnover_windows(year, month, span.end)?;
    let gt = match gt {
        Some(value) => value,
        None => aggregate_turnover(db, previous_fy.0, previous_fy.1).await?,
    };
    let cur_gt = match cur_gt {
        Some(value) => value,
        None => aggregate_turnover(db, current_fy.0, current_fy.1).await?,
    };
    doc.insert("gt".into(), num(gt));
    doc.insert("cur_gt".into(), num(cur_gt));
    doc.insert("version".into(), json!(VERSION));
    doc.insert("hash".into(), json!("hash"));

    let mut b2b: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut b2ba: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut b2cl: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut b2cla: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut b2cs: BTreeMap<B2csKey, Parts> = BTreeMap::new();
    let mut b2csa: BTreeMap<(String, B2csKey), Parts> = BTreeMap::new();
    let mut exp: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut expa: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut ecom_b2b: BTreeMap<(String, String), Vec<Value>> = BTreeMap::new();
    let mut ecom_urp2b: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut ecom_b2c: BTreeMap<EcomKey, Parts> = BTreeMap::new();
    let mut ecom_urp2c: BTreeMap<EcomKey, Parts> = BTreeMap::new();
    let mut supeco_clttx = Vec::new();
    let mut supeco_paytx = Vec::new();
    let mut nil: BTreeMap<String, NilParts> = BTreeMap::new();
    let mut hsn_b2b: BTreeMap<HsnKey, HsnParts> = BTreeMap::new();
    let mut hsn_b2c: BTreeMap<HsnKey, HsnParts> = BTreeMap::new();
    let mut docs = Vec::new();

    for invoice in &invoices {
        let pos = match state_code(&invoice.state) {
            Some(code) => code,
            None => {
                errors.push(format!("invoice {} has no GST state code", invoice.number));
                continue;
            }
        };
        let inter = pos != seller_state;
        let lines_for = lines_of(&lines, invoice.id);
        let classified = match classify_lines(
            invoice,
            &lines_for,
            &line_taxes,
            &header_taxes,
            &components,
            inter,
        ) {
            Ok(rows) => rows,
            Err(error) => {
                errors.push(format!("invoice {}: {error}", invoice.number));
                continue;
            }
        };
        let taxable: Vec<&ClassifiedLine> = classified.iter().filter(|line| line.taxable).collect();
        for line in &classified {
            if line.taxable {
                continue;
            }
            let bucket = nil.entry(supply_ty(inter).to_string()).or_default();
            match line.supply.as_str() {
                "exempt" => bucket.exempt += line.parts.txval,
                "nongst" => bucket.nongst += line.parts.txval,
                _ => bucket.nil += line.parts.txval,
            }
        }
        if taxable.is_empty() && invoice.section == "regular" {
            docs.push(doc_row(&invoice.number, invoice.cancelled));
            continue;
        }
        let b2b_party = !invoice.is_individual && !invoice.gstin.is_empty();
        let hsn_bucket = if b2b_party {
            &mut hsn_b2b
        } else {
            &mut hsn_b2c
        };
        for line in &taxable {
            accumulate_hsn(hsn_bucket, line);
        }
        let value = taxable.iter().fold(Decimal::ZERO, |sum, line| {
            sum + line.parts.txval
                + line.parts.iamt
                + line.parts.camt
                + line.parts.samt
                + line.parts.csamt
        });
        let itms = item_rows(&taxable);
        let amended = invoice.amendment_of_number.is_some();
        match invoice.section.as_str() {
            "export" => {
                let exp_typ = invoice.exp_typ.clone().unwrap_or_default();
                if exp_typ.is_empty() {
                    errors.push(format!(
                        "invoice {} is an export without an export type",
                        invoice.number
                    ));
                    continue;
                }
                let row = export_invoice(invoice, &itms, value);
                if amended {
                    expa.entry(exp_typ).or_default().push(row);
                } else {
                    exp.entry(exp_typ).or_default().push(row);
                }
            }
            "ecom_b2b" => {
                ecom_b2b
                    .entry((invoice.supplier_gstin.clone(), invoice.gstin.clone()))
                    .or_default()
                    .push(plain_invoice(invoice, &itms, value, &pos));
            }
            "ecom_urp2b" => {
                ecom_urp2b
                    .entry(invoice.gstin.clone())
                    .or_default()
                    .push(plain_invoice(invoice, &itms, value, &pos));
            }
            "ecom_b2c" | "ecom_urp2c" => {
                let key = EcomKey {
                    stin: invoice.supplier_gstin.clone(),
                    pos: pos.clone(),
                    inter,
                };
                let map = if invoice.section == "ecom_b2c" {
                    &mut ecom_b2c
                } else {
                    &mut ecom_urp2c
                };
                for line in &taxable {
                    *map.entry(key.clone()).or_default() += line.parts;
                }
            }
            "supeco_clttx" | "supeco_paytx" => {
                let row = supeco_row(invoice, &taxable);
                if invoice.section == "supeco_clttx" {
                    supeco_clttx.push(row);
                } else {
                    supeco_paytx.push(row);
                }
            }
            _ if b2b_party => {
                let row = b2b_invoice(invoice, &itms, value, &pos);
                let map = if amended { &mut b2ba } else { &mut b2b };
                map.entry(invoice.gstin.clone()).or_default().push(row);
            }
            _ if inter && value > Decimal::from(B2CL_LIMIT) => {
                let row = b2cl_invoice(invoice, &itms, value);
                let map = if amended { &mut b2cla } else { &mut b2cl };
                map.entry(pos.clone()).or_default().push(row);
            }
            _ => {
                let typ = if invoice.etin.is_empty() {
                    "OE".to_string()
                } else {
                    "E".to_string()
                };
                for line in &taxable {
                    let key = B2csKey {
                        inter,
                        pos: pos.clone(),
                        etin: invoice.etin.clone(),
                        typ: typ.clone(),
                        rate: line.parts.rate.normalize().to_string(),
                    };
                    if amended {
                        let month = invoice
                            .amendment_of_date
                            .map(|date| format!("{:02}{}", date.month(), date.year()))
                            .unwrap_or_else(|| span.fp.clone());
                        *b2csa.entry((month, key)).or_default() += line.parts;
                    } else {
                        *b2cs.entry(key).or_default() += line.parts;
                    }
                }
            }
        }
        docs.push(doc_row(&invoice.number, invoice.cancelled));
    }

    let mut cdnr: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut cdnra: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut cdnur = Vec::new();
    let mut cdnura = Vec::new();
    let mut note_docs = Vec::new();
    for note in &notes {
        let pos = match state_code(&note.state) {
            Some(code) => code,
            None => {
                errors.push(format!("credit note {} has no GST state code", note.number));
                continue;
            }
        };
        let note_line_rows = lines_of(&note_lines, note.cancelled_id);
        let classified = match classify_lines(
            &note.as_invoice(),
            &note_line_rows,
            &note_taxes,
            &[],
            &components,
            pos != seller_state,
        ) {
            Ok(rows) => rows,
            Err(error) => {
                errors.push(format!("credit note {}: {error}", note.number));
                continue;
            }
        };
        let taxable: Vec<&ClassifiedLine> = classified.iter().filter(|line| line.taxable).collect();
        let value = taxable.iter().fold(Decimal::ZERO, |sum, line| {
            sum + line.parts.txval
                + line.parts.iamt
                + line.parts.camt
                + line.parts.samt
                + line.parts.csamt
        });
        let itms = item_rows(&taxable);
        if note.gstin.is_empty() {
            let row = cdn_unreg(note, &itms, value, &pos);
            if note.original_number.is_some() {
                cdnura.push(row);
            } else {
                cdnur.push(row);
            }
        } else {
            let row = cdn_reg(note, &itms, value, &pos);
            let map = if note.original_number.is_some() {
                &mut cdnra
            } else {
                &mut cdnr
            };
            map.entry(note.gstin.clone()).or_default().push(row);
        }
        note_docs.push(doc_row(&note.number, false));
    }

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }

    insert_ctin(&mut doc, "b2b", b2b);
    insert_ctin(&mut doc, "b2ba", b2ba);
    insert_pos(&mut doc, "b2cl", b2cl);
    insert_pos(&mut doc, "b2cla", b2cla);
    insert_b2cs(&mut doc, b2cs);
    insert_b2csa(&mut doc, b2csa);
    insert_exp(&mut doc, "exp", exp);
    insert_exp(&mut doc, "expa", expa);
    insert_ctin_notes(&mut doc, "cdnr", cdnr);
    insert_ctin_notes(&mut doc, "cdnra", cdnra);
    insert_arr(&mut doc, "cdnur", cdnur);
    insert_arr(&mut doc, "cdnura", cdnura);
    insert_nil(&mut doc, nil);
    insert_hsn(&mut doc, hsn_b2b, hsn_b2c);
    insert_docs(&mut doc, docs, note_docs);
    insert_ecom(&mut doc, ecom_b2b, ecom_b2c, ecom_urp2b, ecom_urp2c);
    insert_supeco(&mut doc, supeco_clttx, supeco_paytx);
    insert_advances(
        &mut doc,
        "at",
        &advances,
        &advance_taxes,
        &components,
        &seller_state,
        false,
    )?;
    insert_adjustments(
        &mut doc,
        &adjustments,
        &advances,
        &advance_taxes,
        &components,
        &seller_state,
    )?;

    serde_json::to_vec_pretty(&Value::Object(doc)).map_err(|e| e.to_string())
}

fn span(year: i32, month: u32) -> Result<Span, String> {
    if !(1..=12).contains(&month) {
        return Err("month must be 1–12".into());
    }
    let offset = ist();
    let start_date = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| format!("invalid return period {month:02}{year}"))?;
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| "invalid return period".to_string())?;
    let start = start_date
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| dt.and_local_timezone(offset).single())
        .ok_or_else(|| "invalid period start".to_string())?
        .with_timezone(&Utc);
    let end = next
        .and_hms_opt(0, 0, 0)
        .and_then(|dt| dt.and_local_timezone(offset).single())
        .ok_or_else(|| "invalid period end".to_string())?
        .with_timezone(&Utc);
    Ok(Span {
        start,
        end,
        fp: format!("{month:02}{year}"),
    })
}

#[expect(clippy::expect_used, reason = "19800 seconds is a valid UTC offset")]
/// Previous financial year, and the current year from 1 April through the return month.
fn turnover_windows(
    year: i32,
    month: u32,
    period_end: DateTime<Utc>,
) -> Result<
    (
        (DateTime<Utc>, DateTime<Utc>),
        (DateTime<Utc>, DateTime<Utc>),
    ),
    String,
> {
    let start_year = if month >= 4 { year } else { year - 1 };
    let previous_start = ist_midnight(start_year - 1, 4, 1)?;
    let current_start = ist_midnight(start_year, 4, 1)?;
    Ok(((previous_start, current_start), (current_start, period_end)))
}

fn ist_midnight(year: i32, month: u32, day: u32) -> Result<DateTime<Utc>, String> {
    let date = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| format!("invalid date {year:04}-{month:02}-{day:02}"))?;
    date.and_hms_opt(0, 0, 0)
        .and_then(|dt| dt.and_local_timezone(ist()).single())
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| format!("invalid date {year:04}-{month:02}-{day:02}"))
}

/// Outward supplies in `[start, end)`, tax excluded, reduced by credit notes in the same window.
async fn aggregate_turnover<C: ConnectionTrait>(
    db: &C,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Decimal, String> {
    let sales = sum_amount(
        db,
        "SELECT COALESCE(SUM(l.pre_tax_amount), 0) AS amount \
         FROM posted_invoice_lines l \
         JOIN posted_invoices i ON i.id = l.posted_invoice_id \
         WHERE i.datetime >= $1 AND i.datetime < $2",
        start,
        end,
    )
    .await?;
    let notes = sum_amount(
        db,
        "SELECT COALESCE(SUM(l.pre_tax_amount), 0) AS amount \
         FROM cancelled_invoice_lines l \
         JOIN cancelled_invoices ci ON ci.id = l.cancelled_invoice_id \
         JOIN credit_notes cn ON cn.id = ci.credit_note_id \
         WHERE cn.datetime >= $1 AND cn.datetime < $2",
        start,
        end,
    )
    .await?;
    Ok(sales - notes)
}

async fn sum_amount<C: ConnectionTrait>(
    db: &C,
    sql: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Decimal, String> {
    let rows = query(db, sql, &[start.into(), end.into()]).await?;
    rows.first()
        .map(|row| col(row, "amount"))
        .transpose()
        .map(|amount| amount.unwrap_or(Decimal::ZERO))
}

fn ist() -> FixedOffset {
    FixedOffset::east_opt(19_800).expect("IST offset")
}

fn state_code(raw: &str) -> Option<String> {
    lariv_plugin_contacts::gst_state::gst_state_code(raw)
}

fn supply_ty(inter: bool) -> &'static str {
    if inter { "INTER" } else { "INTRA" }
}

fn num(value: Decimal) -> Value {
    let text = value.round_dp(2).normalize().to_string();
    serde_json::from_str(&text).unwrap_or(Value::String(text))
}

fn date_text(dt: DateTime<Utc>) -> String {
    let local = dt.with_timezone(&ist());
    format!("{:02}-{:02}-{}", local.day(), local.month(), local.year())
}

fn date_only(date: NaiveDate) -> String {
    format!("{:02}-{:02}-{}", date.day(), date.month(), date.year())
}

async fn seller_gstin<C: ConnectionTrait>(db: &C) -> Result<String, String> {
    let row = db
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT company_gstin FROM invoice_preferences WHERE id = 1".to_string(),
        ))
        .await
        .map_err(|e| e.to_string())?;
    let gstin = row
        .and_then(|row| row.try_get::<Option<String>>("", "company_gstin").ok())
        .flatten()
        .unwrap_or_default();
    let gstin = gstin.trim().to_string();
    if gstin.is_empty() {
        Err("set the company GSTIN on invoice preferences before downloading GSTR-1".into())
    } else {
        Ok(gstin)
    }
}

async fn load_components<C: ConnectionTrait>(db: &C) -> Result<BTreeMap<i64, Component>, String> {
    let rows = query(db, "SELECT tax_id, component FROM gst_tax_components", &[]).await?;
    let mut map = BTreeMap::new();
    for row in rows {
        let id: i64 = col(&row, "tax_id")?;
        let name: String = col(&row, "component")?;
        let component = match name.as_str() {
            "igst" => Component::Igst,
            "cgst" => Component::Cgst,
            "sgst" => Component::Sgst,
            "cess" => Component::Cess,
            _ => continue,
        };
        map.insert(id, component);
    }
    Ok(map)
}

struct InvoiceRow {
    id: i64,
    number: String,
    datetime: DateTime<Utc>,
    is_individual: bool,
    state: String,
    gstin: String,
    section: String,
    inv_typ: String,
    reverse_charge: bool,
    etin: String,
    supplier_gstin: String,
    exp_typ: Option<String>,
    shipping_bill_number: String,
    shipping_bill_date: Option<NaiveDate>,
    port_code: String,
    amendment_of_number: Option<String>,
    amendment_of_date: Option<NaiveDate>,
    cancelled: bool,
}

struct LineRow {
    id: i64,
    parent_id: i64,
    name: String,
    hsn: Option<i64>,
    sac: Option<i64>,
    quantity: Decimal,
    unit: String,
    pre_tax: Decimal,
    supply: String,
}

struct TaxRow {
    parent_id: i64,
    tax_id: i64,
    name: String,
    percentage: Decimal,
    levied: bool,
}

struct NoteRow {
    cancelled_id: i64,
    number: String,
    datetime: DateTime<Utc>,
    gstin: String,
    state: String,
    section: String,
    inv_typ: String,
    reverse_charge: bool,
    exp_typ: Option<String>,
    original_number: Option<String>,
    original_date: Option<NaiveDate>,
}

impl NoteRow {
    fn as_invoice(&self) -> InvoiceRow {
        InvoiceRow {
            id: self.cancelled_id,
            number: self.number.clone(),
            datetime: self.datetime,
            is_individual: self.gstin.is_empty(),
            state: self.state.clone(),
            gstin: self.gstin.clone(),
            section: self.section.clone(),
            inv_typ: self.inv_typ.clone(),
            reverse_charge: self.reverse_charge,
            etin: String::new(),
            supplier_gstin: String::new(),
            exp_typ: self.exp_typ.clone(),
            shipping_bill_number: String::new(),
            shipping_bill_date: None,
            port_code: String::new(),
            amendment_of_number: None,
            amendment_of_date: None,
            cancelled: false,
        }
    }
}

struct AdvanceRow {
    id: i64,
    number: String,
    amount: Decimal,
    pos: String,
    state: String,
}

struct AdjustmentRow {
    payment_id: i64,
    amount: Decimal,
}

#[derive(Clone, Copy, Default)]
struct Parts {
    txval: Decimal,
    rate: Decimal,
    iamt: Decimal,
    camt: Decimal,
    samt: Decimal,
    csamt: Decimal,
}

impl std::ops::AddAssign for Parts {
    fn add_assign(&mut self, rhs: Self) {
        self.txval += rhs.txval;
        self.rate = rhs.rate;
        self.iamt += rhs.iamt;
        self.camt += rhs.camt;
        self.samt += rhs.samt;
        self.csamt += rhs.csamt;
    }
}

#[derive(Clone, Default)]
struct NilParts {
    nil: Decimal,
    exempt: Decimal,
    nongst: Decimal,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct B2csKey {
    inter: bool,
    pos: String,
    etin: String,
    typ: String,
    rate: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EcomKey {
    stin: String,
    pos: String,
    inter: bool,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HsnKey {
    code: String,
    rate: String,
    uqc: String,
}

struct HsnParts {
    desc: String,
    qty: Decimal,
    parts: Parts,
}

struct ClassifiedLine {
    taxable: bool,
    supply: String,
    code: String,
    desc: String,
    uqc: String,
    qty: Decimal,
    parts: Parts,
}

fn lines_of(lines: &[LineRow], parent: i64) -> Vec<&LineRow> {
    lines
        .iter()
        .filter(|line| line.parent_id == parent)
        .collect()
}

fn classify_lines(
    invoice: &InvoiceRow,
    lines: &[&LineRow],
    line_taxes: &[TaxRow],
    header_taxes: &[TaxRow],
    components: &BTreeMap<i64, Component>,
    inter: bool,
) -> Result<Vec<ClassifiedLine>, String> {
    let mut out = Vec::new();
    for line in lines {
        let mut taxes: Vec<&TaxRow> = line_taxes
            .iter()
            .filter(|tax| tax.parent_id == line.id)
            .collect();
        if taxes.is_empty() {
            taxes = header_taxes
                .iter()
                .filter(|tax| tax.parent_id == invoice.id)
                .collect();
        }
        let supply = line.supply.clone();
        let taxable = supply == "taxable";
        let parts = if taxable {
            parts_for(line.pre_tax, &taxes, components, inter)?
        } else {
            Parts {
                txval: line.pre_tax,
                ..Parts::default()
            }
        };
        let code = line
            .hsn
            .or(line.sac)
            .map(|code| code.to_string())
            .unwrap_or_default();
        out.push(ClassifiedLine {
            taxable,
            supply,
            code,
            desc: line.name.clone(),
            uqc: uqc(&line.unit),
            qty: line.quantity,
            parts,
        });
    }
    Ok(out)
}

fn parts_for(
    txval: Decimal,
    taxes: &[&TaxRow],
    components: &BTreeMap<i64, Component>,
    inter: bool,
) -> Result<Parts, String> {
    let mut parts = Parts {
        txval,
        ..Parts::default()
    };
    for tax in taxes {
        if !tax.levied {
            continue;
        }
        let component = components
            .get(&tax.tax_id)
            .copied()
            .ok_or_else(|| format!("tax {} is not mapped to a GST component", tax.name))?;
        let amount = (txval * tax.percentage / Decimal::from(100)).round_dp(2);
        match component {
            Component::Igst => {
                parts.iamt += amount;
                parts.rate += tax.percentage;
            }
            Component::Cgst => {
                parts.camt += amount;
                parts.rate += tax.percentage;
            }
            Component::Sgst => {
                parts.samt += amount;
                parts.rate += tax.percentage;
            }
            Component::Cess => parts.csamt += amount,
        }
    }
    if inter {
        parts.camt = Decimal::ZERO;
        parts.samt = Decimal::ZERO;
    } else {
        parts.iamt = Decimal::ZERO;
    }
    Ok(parts)
}

fn item_rows(lines: &[&ClassifiedLine]) -> Vec<Value> {
    let mut grouped: BTreeMap<String, Parts> = BTreeMap::new();
    for line in lines {
        let key = line.parts.rate.normalize().to_string();
        *grouped.entry(key).or_default() += line.parts;
    }
    grouped
        .into_iter()
        .enumerate()
        .map(|(index, (_, parts))| {
            json!({
                "num": index + 1,
                "itm_det": item_det(&parts),
            })
        })
        .collect()
}

fn item_det(parts: &Parts) -> Value {
    json!({
        "txval": num(parts.txval),
        "rt": num(parts.rate),
        "iamt": num(parts.iamt),
        "camt": num(parts.camt),
        "samt": num(parts.samt),
        "csamt": num(parts.csamt),
    })
}

fn b2b_invoice(invoice: &InvoiceRow, itms: &[Value], value: Decimal, pos: &str) -> Value {
    let mut row = Map::new();
    row.insert("inum".into(), json!(invoice.number));
    row.insert("idt".into(), json!(date_text(invoice.datetime)));
    row.insert("val".into(), num(value));
    row.insert("pos".into(), json!(pos));
    row.insert(
        "rchrg".into(),
        json!(if invoice.reverse_charge { "Y" } else { "N" }),
    );
    row.insert("inv_typ".into(), json!(invoice.inv_typ));
    if !invoice.etin.is_empty() {
        row.insert("etin".into(), json!(invoice.etin));
    }
    if let Some(number) = &invoice.amendment_of_number {
        row.insert("oinum".into(), json!(number));
        if let Some(date) = invoice.amendment_of_date {
            row.insert("oidt".into(), json!(date_only(date)));
        }
    }
    row.insert("itms".into(), json!(itms));
    Value::Object(row)
}

fn b2cl_invoice(invoice: &InvoiceRow, itms: &[Value], value: Decimal) -> Value {
    let mut row = b2b_invoice(invoice, itms, value, "");
    if let Value::Object(map) = &mut row {
        map.remove("pos");
        map.remove("rchrg");
        map.remove("inv_typ");
    }
    row
}

fn plain_invoice(invoice: &InvoiceRow, itms: &[Value], value: Decimal, pos: &str) -> Value {
    b2b_invoice(invoice, itms, value, pos)
}

fn export_invoice(invoice: &InvoiceRow, itms: &[Value], value: Decimal) -> Value {
    let mut row = Map::new();
    row.insert("inum".into(), json!(invoice.number));
    row.insert("idt".into(), json!(date_text(invoice.datetime)));
    row.insert("val".into(), num(value));
    if !invoice.shipping_bill_number.is_empty() {
        row.insert("sbnum".into(), json!(invoice.shipping_bill_number));
    }
    if let Some(date) = invoice.shipping_bill_date {
        row.insert("sbdt".into(), json!(date_only(date)));
    }
    if !invoice.port_code.is_empty() {
        row.insert("sbpcode".into(), json!(invoice.port_code));
    }
    if let Some(number) = &invoice.amendment_of_number {
        row.insert("oinum".into(), json!(number));
        if let Some(date) = invoice.amendment_of_date {
            row.insert("oidt".into(), json!(date_only(date)));
        }
    }
    row.insert("itms".into(), json!(itms));
    Value::Object(row)
}

fn cdn_reg(note: &NoteRow, itms: &[Value], value: Decimal, pos: &str) -> Value {
    let mut row = Map::new();
    row.insert("ntty".into(), json!("C"));
    row.insert("nt_num".into(), json!(note.number));
    row.insert("nt_dt".into(), json!(date_text(note.datetime)));
    row.insert("pos".into(), json!(pos));
    row.insert(
        "rchrg".into(),
        json!(if note.reverse_charge { "Y" } else { "N" }),
    );
    row.insert("inv_typ".into(), json!(note.inv_typ));
    row.insert("val".into(), num(value));
    if let Some(number) = &note.original_number {
        row.insert("ont_num".into(), json!(number));
        if let Some(date) = note.original_date {
            row.insert("ont_dt".into(), json!(date_only(date)));
        }
    }
    row.insert("itms".into(), json!(itms));
    Value::Object(row)
}

fn cdn_unreg(note: &NoteRow, itms: &[Value], value: Decimal, pos: &str) -> Value {
    let typ = match note.exp_typ.as_deref() {
        Some("WPAY") => "EXPWP",
        Some("WOPAY") => "EXPWOP",
        _ => "B2CL",
    };
    let mut row = cdn_reg(note, itms, value, pos);
    if let Value::Object(map) = &mut row {
        map.insert("typ".into(), json!(typ));
        map.remove("rchrg");
        map.remove("inv_typ");
    }
    row
}

fn supeco_row(invoice: &InvoiceRow, lines: &[&ClassifiedLine]) -> Value {
    let mut parts = Parts::default();
    for line in lines {
        parts += line.parts;
    }
    json!({
        "etin": invoice.etin,
        "suppval": num(parts.txval),
        "igst": num(parts.iamt),
        "cgst": num(parts.camt),
        "sgst": num(parts.samt),
        "cess": num(parts.csamt),
    })
}

fn accumulate_hsn(map: &mut BTreeMap<HsnKey, HsnParts>, line: &ClassifiedLine) {
    let key = HsnKey {
        code: line.code.clone(),
        rate: line.parts.rate.normalize().to_string(),
        uqc: line.uqc.clone(),
    };
    let entry = map.entry(key).or_insert_with(|| HsnParts {
        desc: line.desc.clone(),
        qty: Decimal::ZERO,
        parts: Parts::default(),
    });
    entry.qty += line.qty;
    entry.parts += line.parts;
}

fn uqc(unit: &str) -> String {
    match unit.trim().to_ascii_lowercase().as_str() {
        "number" | "nos" | "pcs" | "piece" | "pieces" => "NOS".into(),
        "kg" | "kgs" | "kilogram" | "kilograms" => "KGS".into(),
        "m" | "meter" | "metre" | "meters" | "metres" => "MTR".into(),
        "l" | "ltr" | "litre" | "liter" => "LTR".into(),
        "" => "OTH".into(),
        _ => "OTH".into(),
    }
}

fn doc_row(number: &str, cancelled: bool) -> Value {
    let cancel = if cancelled { 1 } else { 0 };
    json!({
        "num": 1,
        "from": number,
        "to": number,
        "totnum": 1,
        "cancel": cancel,
        "net_issue": 1 - cancel,
    })
}

fn insert_ctin(doc: &mut Map<String, Value>, key: &str, groups: BTreeMap<String, Vec<Value>>) {
    let rows: Vec<Value> = groups
        .into_iter()
        .filter(|(_, inv)| !inv.is_empty())
        .map(|(ctin, inv)| json!({"ctin": ctin, "inv": inv}))
        .collect();
    insert_arr(doc, key, rows);
}

fn insert_pos(doc: &mut Map<String, Value>, key: &str, groups: BTreeMap<String, Vec<Value>>) {
    let rows: Vec<Value> = groups
        .into_iter()
        .filter(|(_, inv)| !inv.is_empty())
        .map(|(pos, inv)| json!({"pos": pos, "inv": inv}))
        .collect();
    insert_arr(doc, key, rows);
}

fn insert_exp(doc: &mut Map<String, Value>, key: &str, groups: BTreeMap<String, Vec<Value>>) {
    let rows: Vec<Value> = groups
        .into_iter()
        .filter(|(_, inv)| !inv.is_empty())
        .map(|(exp_typ, inv)| json!({"exp_typ": exp_typ, "inv": inv}))
        .collect();
    insert_arr(doc, key, rows);
}

fn insert_ctin_notes(
    doc: &mut Map<String, Value>,
    key: &str,
    groups: BTreeMap<String, Vec<Value>>,
) {
    let rows: Vec<Value> = groups
        .into_iter()
        .filter(|(_, notes)| !notes.is_empty())
        .map(|(ctin, nt)| json!({"ctin": ctin, "nt": nt}))
        .collect();
    insert_arr(doc, key, rows);
}

fn insert_arr(doc: &mut Map<String, Value>, key: &str, rows: Vec<Value>) {
    if !rows.is_empty() {
        doc.insert(key.into(), Value::Array(rows));
    }
}

fn insert_b2cs(doc: &mut Map<String, Value>, groups: BTreeMap<B2csKey, Parts>) {
    let rows: Vec<Value> = groups
        .into_iter()
        .map(|(key, parts)| {
            json!({
                "sply_ty": supply_ty(key.inter),
                "rt": num(parts.rate),
                "typ": key.typ,
                "etin": key.etin,
                "pos": key.pos,
                "txval": num(parts.txval),
                "iamt": num(parts.iamt),
                "camt": num(parts.camt),
                "samt": num(parts.samt),
                "csamt": num(parts.csamt),
            })
        })
        .collect();
    insert_arr(doc, "b2cs", rows);
}

fn insert_b2csa(doc: &mut Map<String, Value>, groups: BTreeMap<(String, B2csKey), Parts>) {
    insert_arr(doc, "b2csa", groups_b2csa_rows(groups));
}

fn groups_b2csa_rows(groups: BTreeMap<(String, B2csKey), Parts>) -> Vec<Value> {
    let mut buckets: BTreeMap<(String, bool, String, String, String), Vec<Value>> = BTreeMap::new();
    for ((omon, key), parts) in groups {
        buckets
            .entry((omon, key.inter, key.pos, key.etin, key.typ))
            .or_default()
            .push(json!({
                "rt": num(parts.rate),
                "txval": num(parts.txval),
                "iamt": num(parts.iamt),
                "camt": num(parts.camt),
                "samt": num(parts.samt),
                "csamt": num(parts.csamt),
            }));
    }
    buckets
        .into_iter()
        .map(|((omon, inter, pos, etin, typ), itms)| {
            json!({
                "omon": omon,
                "sply_ty": supply_ty(inter),
                "typ": typ,
                "etin": etin,
                "pos": pos,
                "itms": itms,
            })
        })
        .collect()
}

fn insert_nil(doc: &mut Map<String, Value>, groups: BTreeMap<String, NilParts>) {
    let rows: Vec<Value> = groups
        .into_iter()
        .map(|(sply, parts)| {
            json!({
                "sply_ty": sply,
                "expt_amt": num(parts.exempt),
                "nil_amt": num(parts.nil),
                "ngsup_amt": num(parts.nongst),
            })
        })
        .collect();
    if !rows.is_empty() {
        doc.insert("nil".into(), json!({"inv": rows}));
    }
}

fn insert_hsn(
    doc: &mut Map<String, Value>,
    b2b: BTreeMap<HsnKey, HsnParts>,
    b2c: BTreeMap<HsnKey, HsnParts>,
) {
    let b2b_rows = hsn_rows(b2b);
    let b2c_rows = hsn_rows(b2c);
    let mut data = b2b_rows.clone();
    data.extend(b2c_rows.iter().cloned());
    if data.is_empty() {
        return;
    }
    doc.insert(
        "hsn".into(),
        json!({
            "data": data,
            "hsn_b2b": b2b_rows,
            "hsn_b2c": b2c_rows,
        }),
    );
}

fn hsn_rows(groups: BTreeMap<HsnKey, HsnParts>) -> Vec<Value> {
    groups
        .into_iter()
        .enumerate()
        .map(|(index, (key, parts))| {
            json!({
                "num": index + 1,
                "hsn_sc": key.code,
                "desc": parts.desc,
                "uqc": key.uqc,
                "qty": num(parts.qty),
                "rt": num(parts.parts.rate),
                "txval": num(parts.parts.txval),
                "iamt": num(parts.parts.iamt),
                "camt": num(parts.parts.camt),
                "samt": num(parts.parts.samt),
                "csamt": num(parts.parts.csamt),
            })
        })
        .collect()
}

fn insert_docs(doc: &mut Map<String, Value>, invoices: Vec<Value>, notes: Vec<Value>) {
    let mut det = Vec::new();
    if !invoices.is_empty() {
        det.push(json!({
            "doc_num": 1,
            "doc_typ": "Invoices for outward supply",
            "docs": invoices,
        }));
    }
    if !notes.is_empty() {
        det.push(json!({
            "doc_num": 5,
            "doc_typ": "Credit Note",
            "docs": notes,
        }));
    }
    if !det.is_empty() {
        doc.insert("doc_issue".into(), json!({"doc_det": det}));
    }
}

fn insert_ecom(
    doc: &mut Map<String, Value>,
    b2b: BTreeMap<(String, String), Vec<Value>>,
    b2c: BTreeMap<EcomKey, Parts>,
    urp2b: BTreeMap<String, Vec<Value>>,
    urp2c: BTreeMap<EcomKey, Parts>,
) {
    let mut section = Map::new();
    let b2b_rows: Vec<Value> = b2b
        .into_iter()
        .map(|((stin, rtin), inv)| json!({"stin": stin, "rtin": rtin, "inv": inv}))
        .collect();
    if !b2b_rows.is_empty() {
        section.insert("b2b".into(), Value::Array(b2b_rows));
    }
    let b2c_rows: Vec<Value> = b2c
        .into_iter()
        .map(|(key, parts)| ecom_summary(&key, &parts))
        .collect();
    if !b2c_rows.is_empty() {
        section.insert("b2c".into(), Value::Array(b2c_rows));
    }
    let urp2b_rows: Vec<Value> = urp2b
        .into_iter()
        .map(|(rtin, inv)| json!({"rtin": rtin, "inv": inv}))
        .collect();
    if !urp2b_rows.is_empty() {
        section.insert("urp2b".into(), Value::Array(urp2b_rows));
    }
    let urp2c_rows: Vec<Value> = urp2c
        .into_iter()
        .map(|(key, parts)| ecom_summary(&key, &parts))
        .collect();
    if !urp2c_rows.is_empty() {
        section.insert("urp2c".into(), Value::Array(urp2c_rows));
    }
    if !section.is_empty() {
        doc.insert("ecom".into(), Value::Object(section));
    }
}

fn ecom_summary(key: &EcomKey, parts: &Parts) -> Value {
    json!({
        "stin": key.stin,
        "pos": key.pos,
        "sply_ty": supply_ty(key.inter),
        "rt": num(parts.rate),
        "txval": num(parts.txval),
        "iamt": num(parts.iamt),
        "camt": num(parts.camt),
        "samt": num(parts.samt),
        "csamt": num(parts.csamt),
    })
}

fn insert_supeco(doc: &mut Map<String, Value>, clttx: Vec<Value>, paytx: Vec<Value>) {
    if clttx.is_empty() && paytx.is_empty() {
        return;
    }
    let mut section = Map::new();
    if !clttx.is_empty() {
        section.insert("clttx".into(), Value::Array(clttx));
    }
    if !paytx.is_empty() {
        section.insert("paytx".into(), Value::Array(paytx));
    }
    doc.insert("supeco".into(), Value::Object(section));
}

fn insert_advances(
    doc: &mut Map<String, Value>,
    key: &str,
    advances: &[AdvanceRow],
    taxes: &[TaxRow],
    components: &BTreeMap<i64, Component>,
    seller_state: &str,
    _adjusted: bool,
) -> Result<(), String> {
    let mut groups: BTreeMap<(String, bool), BTreeMap<String, Parts>> = BTreeMap::new();
    for advance in advances {
        let pos = state_code(&advance.pos)
            .or_else(|| state_code(&advance.state))
            .ok_or_else(|| format!("advance {} has no place of supply", advance.number))?;
        let inter = pos != seller_state;
        let tax_rows: Vec<&TaxRow> = taxes
            .iter()
            .filter(|tax| tax.parent_id == advance.id)
            .collect();
        let rate_sum: Decimal = tax_rows
            .iter()
            .filter(|tax| tax.levied)
            .map(|tax| tax.percentage)
            .sum();
        if rate_sum.is_zero() {
            return Err(format!(
                "advance {} needs GST taxes before it can be exported",
                advance.number
            ));
        }
        let taxable =
            (advance.amount * Decimal::from(100) / (Decimal::from(100) + rate_sum)).round_dp(2);
        let parts = parts_for(taxable, &tax_rows, components, inter)?;
        let rate = parts.rate.normalize().to_string();
        let mut gross = parts;
        gross.txval = advance.amount;
        *groups
            .entry((pos, inter))
            .or_default()
            .entry(rate)
            .or_default() += gross;
    }
    let rows: Vec<Value> = groups
        .into_iter()
        .map(|((pos, inter), items)| {
            let itms: Vec<Value> = items
                .into_values()
                .map(|parts| {
                    json!({
                        "rt": num(parts.rate),
                        "ad_amt": num(parts.txval),
                        "iamt": num(parts.iamt),
                        "camt": num(parts.camt),
                        "samt": num(parts.samt),
                        "csamt": num(parts.csamt),
                    })
                })
                .collect();
            json!({"pos": pos, "sply_ty": supply_ty(inter), "itms": itms})
        })
        .collect();
    insert_arr(doc, key, rows);
    Ok(())
}

fn insert_adjustments(
    doc: &mut Map<String, Value>,
    adjustments: &[AdjustmentRow],
    advances: &[AdvanceRow],
    taxes: &[TaxRow],
    components: &BTreeMap<i64, Component>,
    seller_state: &str,
) -> Result<(), String> {
    let mut synthetic = Vec::new();
    for adjustment in adjustments {
        let Some(advance) = advances.iter().find(|row| row.id == adjustment.payment_id) else {
            return Err(format!(
                "advance adjustment on payment {} has no matching advance",
                adjustment.payment_id
            ));
        };
        synthetic.push(AdvanceRow {
            id: advance.id,
            number: advance.number.clone(),
            amount: adjustment.amount,
            pos: advance.pos.clone(),
            state: advance.state.clone(),
        });
    }
    insert_advances(
        doc,
        "txpd",
        &synthetic,
        taxes,
        components,
        seller_state,
        true,
    )
}

async fn load_invoices<C: ConnectionTrait>(db: &C, span: &Span) -> Result<Vec<InvoiceRow>, String> {
    let rows = query(
        db,
        "SELECT i.id, i.number, i.datetime, c.is_individual, \
         COALESCE(NULLIF(TRIM(c.state), ''), NULLIF(TRIM(co.state), ''), ''), \
         COALESCE(co.gstin, ''), COALESCE(g.gst_section, 'regular'), COALESCE(g.gst_inv_typ, 'R'), \
         COALESCE(g.reverse_charge, false), COALESCE(g.etin, ''), COALESCE(g.supplier_gstin, ''), \
         g.exp_typ, COALESCE(g.shipping_bill_number, ''), g.shipping_bill_date, \
         COALESCE(g.port_code, ''), g.amendment_of_number, g.amendment_of_date, \
         EXISTS(SELECT 1 FROM cancelled_invoices ci WHERE ci.posted_invoice_id = i.id) AS cancelled \
         FROM posted_invoices i \
         LEFT JOIN customers c ON c.id = i.customer_id \
         LEFT JOIN crm_companies co ON co.id = c.company_id \
         LEFT JOIN posted_invoice_gst_details g ON g.posted_invoice_id = i.id \
         WHERE i.datetime >= $1 AND i.datetime < $2 \
         ORDER BY i.number",
        &[span.start.into(), span.end.into()],
    )
    .await?;
    rows.iter()
        .map(|row| {
            Ok(InvoiceRow {
                id: col(row, "id")?,
                number: col(row, "number")?,
                datetime: col(row, "datetime")?,
                is_individual: col(row, "is_individual")?,
                state: col_at(row, 4)?,
                gstin: col_at(row, 5)?,
                section: col_at(row, 6)?,
                inv_typ: col_at(row, 7)?,
                reverse_charge: col_at(row, 8)?,
                etin: col_at(row, 9)?,
                supplier_gstin: col_at(row, 10)?,
                exp_typ: col_at(row, 11)?,
                shipping_bill_number: col_at(row, 12)?,
                shipping_bill_date: col_at(row, 13)?,
                port_code: col_at(row, 14)?,
                amendment_of_number: col_at(row, 15)?,
                amendment_of_date: col_at(row, 16)?,
                cancelled: col_at(row, 17)?,
            })
        })
        .collect()
}

async fn load_lines<C: ConnectionTrait>(db: &C, span: &Span) -> Result<Vec<LineRow>, String> {
    load_line_query(
        db,
        span,
        "SELECT l.id, l.posted_invoice_id, l.product_name, l.hsn_code, l.sac_code, l.quantity, \
         l.qty_unit, l.pre_tax_amount, COALESCE(l.gst_supply, 'taxable') \
         FROM posted_invoice_lines l \
         JOIN posted_invoices i ON i.id = l.posted_invoice_id \
         WHERE i.datetime >= $1 AND i.datetime < $2 ORDER BY l.id",
    )
    .await
}

async fn load_note_lines<C: ConnectionTrait>(db: &C, span: &Span) -> Result<Vec<LineRow>, String> {
    load_line_query(
        db,
        span,
        "SELECT l.id, l.cancelled_invoice_id, l.product_name, l.hsn_code, l.sac_code, l.quantity, \
         l.qty_unit, l.pre_tax_amount, COALESCE(l.gst_supply, 'taxable') \
         FROM cancelled_invoice_lines l \
         JOIN cancelled_invoices ci ON ci.id = l.cancelled_invoice_id \
         JOIN credit_notes cn ON cn.id = ci.credit_note_id \
         WHERE cn.datetime >= $1 AND cn.datetime < $2 ORDER BY l.id",
    )
    .await
}

async fn load_line_query<C: ConnectionTrait>(
    db: &C,
    span: &Span,
    sql: &str,
) -> Result<Vec<LineRow>, String> {
    let rows = query(db, sql, &[span.start.into(), span.end.into()]).await?;
    rows.iter()
        .map(|row| {
            Ok(LineRow {
                id: col_at(row, 0)?,
                parent_id: col_at(row, 1)?,
                name: col_at(row, 2)?,
                hsn: col_at(row, 3)?,
                sac: col_at(row, 4)?,
                quantity: col_at(row, 5)?,
                unit: col_at(row, 6)?,
                pre_tax: col_at(row, 7)?,
                supply: col_at(row, 8)?,
            })
        })
        .collect()
}

async fn load_taxes<C: ConnectionTrait>(
    db: &C,
    span: &Span,
    table: &str,
    parent: &str,
) -> Result<Vec<TaxRow>, String> {
    let sql = format!(
        "SELECT lt.{parent}, t.id, t.name, t.percentage, t.tax_type::text \
         FROM {table} lt \
         JOIN taxes t ON t.id = lt.tax_id \
         JOIN posted_invoices i ON i.id = lt.{parent} OR EXISTS ( \
            SELECT 1 FROM posted_invoice_lines l WHERE l.id = lt.{parent} AND l.posted_invoice_id = i.id \
         ) \
         WHERE i.datetime >= $1 AND i.datetime < $2"
    );
    let _ = (table, parent);
    let rows = query(db, &sql, &[span.start.into(), span.end.into()]).await?;
    rows.iter().map(tax_row).collect()
}

async fn load_note_taxes<C: ConnectionTrait>(db: &C, span: &Span) -> Result<Vec<TaxRow>, String> {
    let rows = query(
        db,
        "SELECT lt.cancelled_invoice_line_id, t.id, t.name, t.percentage, t.tax_type::text \
         FROM cancelled_invoice_line_taxes lt \
         JOIN taxes t ON t.id = lt.tax_id \
         JOIN cancelled_invoice_lines l ON l.id = lt.cancelled_invoice_line_id \
         JOIN cancelled_invoices ci ON ci.id = l.cancelled_invoice_id \
         JOIN credit_notes cn ON cn.id = ci.credit_note_id \
         WHERE cn.datetime >= $1 AND cn.datetime < $2",
        &[span.start.into(), span.end.into()],
    )
    .await?;
    rows.iter().map(tax_row).collect()
}

fn tax_row(row: &QueryResult) -> Result<TaxRow, String> {
    let kind: String = col_at(row, 4)?;
    Ok(TaxRow {
        parent_id: col_at(row, 0)?,
        tax_id: col_at(row, 1)?,
        name: col_at(row, 2)?,
        percentage: col_at(row, 3)?,
        levied: kind == "levied",
    })
}

async fn load_notes<C: ConnectionTrait>(db: &C, span: &Span) -> Result<Vec<NoteRow>, String> {
    let rows = query(
        db,
        "SELECT ci.id, cn.credit_note_number, cn.datetime, COALESCE(co.gstin, ''), \
         COALESCE(NULLIF(TRIM(c.state), ''), NULLIF(TRIM(co.state), ''), ''), \
         COALESCE(g.gst_section, 'regular'), COALESCE(g.gst_inv_typ, 'R'), \
         COALESCE(g.reverse_charge, false), g.exp_typ, cn.original_note_number, cn.original_note_date \
         FROM credit_notes cn \
         JOIN cancelled_invoices ci ON ci.credit_note_id = cn.id \
         LEFT JOIN customers c ON c.id = ci.customer_id \
         LEFT JOIN crm_companies co ON co.id = c.company_id \
         LEFT JOIN cancelled_invoice_gst_details g ON g.cancelled_invoice_id = ci.id \
         WHERE cn.datetime >= $1 AND cn.datetime < $2 \
         ORDER BY cn.credit_note_number",
        &[span.start.into(), span.end.into()],
    )
    .await?;
    rows.iter()
        .map(|row| {
            Ok(NoteRow {
                cancelled_id: col_at(row, 0)?,
                number: col_at(row, 1)?,
                datetime: col_at(row, 2)?,
                gstin: col_at(row, 3)?,
                state: col_at(row, 4)?,
                section: col_at(row, 5)?,
                inv_typ: col_at(row, 6)?,
                reverse_charge: col_at(row, 7)?,
                exp_typ: col_at(row, 8)?,
                original_number: col_at(row, 9)?,
                original_date: col_at(row, 10)?,
            })
        })
        .collect()
}

async fn load_advances<C: ConnectionTrait>(db: &C, span: &Span) -> Result<Vec<AdvanceRow>, String> {
    let rows = query(
        db,
        "SELECT p.id, p.number, p.amount, COALESCE(p.gst_pos, ''), \
         COALESCE(NULLIF(TRIM(c.state), ''), NULLIF(TRIM(co.state), ''), '') \
         FROM payments p \
         LEFT JOIN customers c ON c.id = p.customer_id \
         LEFT JOIN crm_companies co ON co.id = c.company_id \
         WHERE p.is_gst_advance AND p.datetime >= $1 AND p.datetime < $2 \
         ORDER BY p.number",
        &[span.start.into(), span.end.into()],
    )
    .await?;
    rows.iter()
        .map(|row| {
            Ok(AdvanceRow {
                id: col_at(row, 0)?,
                number: col_at(row, 1)?,
                amount: col_at(row, 2)?,
                pos: col_at(row, 3)?,
                state: col_at(row, 4)?,
            })
        })
        .collect()
}

async fn load_adjustments<C: ConnectionTrait>(
    db: &C,
    span: &Span,
) -> Result<Vec<AdjustmentRow>, String> {
    let start = span.start.date_naive();
    let end = span.end.date_naive();
    let rows = query(
        db,
        "SELECT payment_id, gross_amount, adjusted_on FROM gst_advance_adjustments \
         WHERE adjusted_on >= $1 AND adjusted_on < $2",
        &[start.into(), end.into()],
    )
    .await?;
    rows.iter()
        .map(|row| {
            Ok(AdjustmentRow {
                payment_id: col_at(row, 0)?,
                amount: col_at(row, 1)?,
            })
        })
        .collect()
}

async fn load_advance_taxes<C: ConnectionTrait>(db: &C) -> Result<Vec<TaxRow>, String> {
    let rows = query(
        db,
        "SELECT lt.payment_id, t.id, t.name, t.percentage, t.tax_type::text \
         FROM payment_gst_taxes lt JOIN taxes t ON t.id = lt.tax_id",
        &[],
    )
    .await?;
    rows.iter().map(tax_row).collect()
}

async fn query<C: ConnectionTrait>(
    db: &C,
    sql: &str,
    values: &[DbValue],
) -> Result<Vec<QueryResult>, String> {
    db.query_all_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        values.to_vec(),
    ))
    .await
    .map_err(|e| e.to_string())
}

fn col<T>(row: &QueryResult, name: &str) -> Result<T, String>
where
    T: sea_orm::TryGetable,
{
    row.try_get("", name).map_err(|e| e.to_string())
}

fn col_at<T>(row: &QueryResult, index: usize) -> Result<T, String>
where
    T: sea_orm::TryGetable,
{
    row.try_get_by_index(index).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::state_code;

    #[test]
    fn company_state_names_become_gst_codes() {
        assert_eq!(state_code("Maharashtra").as_deref(), Some("27"));
        assert_eq!(state_code("07").as_deref(), Some("07"));
        assert_eq!(state_code("29 — KARNATAKA").as_deref(), Some("29"));
        assert_eq!(state_code("27AAAAA0000A1Z5").as_deref(), Some("27"));
        assert_eq!(state_code("").as_deref(), None);
    }
}
