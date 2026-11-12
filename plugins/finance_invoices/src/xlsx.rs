//! Posted-invoice GST workbook: invoice rows and HSN lines.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_xlsxwriter::{Format, Workbook, Worksheet};
use sea_orm::{ConnectionTrait, DatabaseBackend, QueryResult, Statement};

const EXCEL_MAX_CHARS: usize = 32_767;

/// Build the GST workbook for every posted invoice.
pub async fn build_invoice_workbook<C: ConnectionTrait>(db: &C) -> Result<Vec<u8>, String> {
    let loaded = load(db).await?;
    write_workbook(&assemble(loaded))
}

struct Loaded {
    invoices: Vec<InvoiceRow>,
    lines: Vec<LineRow>,
    line_taxes: Vec<TaxLink>,
    header_taxes: Vec<TaxLink>,
    terms: Vec<TermRow>,
    paid_invoice_ids: HashSet<i64>,
}

struct InvoiceRow {
    id: i64,
    number: String,
    datetime: DateTime<Utc>,
    term_id: Option<i64>,
    is_individual: bool,
    state: String,
    company_name: String,
    gstin: String,
    contact_name: String,
}

struct LineRow {
    id: i64,
    invoice_id: i64,
    product_name: String,
    hsn_code: Option<i64>,
    sac_code: Option<i64>,
    quantity: Decimal,
    qty_unit: String,
    pre_tax: Decimal,
}

struct TaxLink {
    parent_id: i64,
    tax_id: i64,
    percentage: Decimal,
    levied: bool,
}

struct TermRow {
    term_id: i64,
    line_order: i32,
    id: i64,
    due_date: NaiveDate,
}

struct Slice {
    rate_pct: Decimal,
    basic: Decimal,
    tax: Decimal,
}

struct HsnLine {
    code: Option<i64>,
    description: String,
    uqc: String,
    quantity: Decimal,
    total_value: Decimal,
    rate_fraction: Decimal,
    taxable: Decimal,
    central: Decimal,
    state: Decimal,
}

struct ExportInvoice {
    number: String,
    date: NaiveDate,
    name: String,
    gstin: String,
    state: String,
    is_individual: bool,
    receipt: bool,
    payment_term: String,
    invoice_value: Decimal,
    slices: Vec<Slice>,
    hsn: Vec<HsnLine>,
}

async fn load<C: ConnectionTrait>(db: &C) -> Result<Loaded, String> {
    let backend = db.get_database_backend();
    let invoices = query(db, backend, invoice_sql()).await?;
    let lines = query(db, backend, line_sql()).await?;
    let tax_type = tax_type_sql(backend);
    let line_taxes = query(
        db,
        backend,
        &tax_sql(
            "posted_invoice_line_taxes",
            "posted_invoice_line_id",
            tax_type,
        ),
    )
    .await?;
    let header_taxes = query(
        db,
        backend,
        &tax_sql("posted_invoice_taxes", "posted_invoice_id", tax_type),
    )
    .await?;
    let terms = query(db, backend, term_sql()).await?;
    let paid = query(db, backend, paid_sql()).await?;
    Ok(Loaded {
        invoices: invoices
            .iter()
            .map(invoice_row)
            .collect::<Result<Vec<_>, _>>()?,
        lines: lines.iter().map(line_row).collect::<Result<Vec<_>, _>>()?,
        line_taxes: line_taxes.iter().map(tax_link).collect(),
        header_taxes: header_taxes.iter().map(tax_link).collect(),
        terms: terms.iter().filter_map(term_row).collect(),
        paid_invoice_ids: paid.iter().filter_map(|row| col_i64(row, 0)).collect(),
    })
}

fn invoice_sql() -> &'static str {
    "SELECT i.id, i.number, i.datetime, i.posted_payment_term_id, \
            c.is_individual, c.state, co.name, co.gstin, ct.name \
     FROM posted_invoices i \
     LEFT JOIN customers c ON c.id = i.customer_id \
     LEFT JOIN crm_companies co ON co.id = c.company_id \
     LEFT JOIN crm_contacts ct ON ct.id = c.individual_id"
}

fn line_sql() -> &'static str {
    "SELECT id, posted_invoice_id, product_name, hsn_code, sac_code, \
            quantity, qty_unit, pre_tax_amount \
     FROM posted_invoice_lines"
}

fn tax_sql(table: &str, parent: &str, tax_type: &str) -> String {
    format!(
        "SELECT lt.{parent}, t.id, t.percentage, {tax_type} \
         FROM {table} lt \
         JOIN taxes t ON t.id = lt.tax_id"
    )
}

fn term_sql() -> &'static str {
    "SELECT id, posted_payment_term_id, line_order, due_date \
     FROM posted_payment_term_lines"
}

fn paid_sql() -> &'static str {
    "SELECT posted_invoice_id FROM paid_invoices \
     UNION \
     SELECT posted_invoice_id FROM partially_paid_invoices"
}

fn tax_type_sql(backend: DatabaseBackend) -> &'static str {
    match backend {
        DatabaseBackend::Postgres => "t.tax_type::text",
        _ => "CAST(t.tax_type AS TEXT)",
    }
}

async fn query<C: ConnectionTrait>(
    db: &C,
    backend: DatabaseBackend,
    sql: &str,
) -> Result<Vec<QueryResult>, String> {
    db.query_all_raw(Statement::from_string(backend, sql.to_string()))
        .await
        .map_err(|err| err.to_string())
}

fn assemble(loaded: Loaded) -> Vec<ExportInvoice> {
    let mut lines_by_invoice: HashMap<i64, Vec<LineRow>> = HashMap::new();
    for line in loaded.lines {
        lines_by_invoice
            .entry(line.invoice_id)
            .or_default()
            .push(line);
    }
    for lines in lines_by_invoice.values_mut() {
        lines.sort_by_key(|line| line.id);
    }
    let mut terms_by_id: HashMap<i64, Vec<TermRow>> = HashMap::new();
    for term in loaded.terms {
        terms_by_id.entry(term.term_id).or_default().push(term);
    }
    for terms in terms_by_id.values_mut() {
        terms.sort_by(|left, right| {
            left.line_order
                .cmp(&right.line_order)
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    let mut invoices = loaded.invoices;
    invoices.sort_by(|left, right| {
        india_date(left.datetime)
            .cmp(&india_date(right.datetime))
            .then_with(|| left.number.cmp(&right.number))
            .then_with(|| left.id.cmp(&right.id))
    });

    invoices
        .into_iter()
        .map(|invoice| {
            export_invoice(
                invoice,
                &lines_by_invoice,
                &loaded.line_taxes,
                &loaded.header_taxes,
                &terms_by_id,
                &loaded.paid_invoice_ids,
            )
        })
        .collect()
}

fn export_invoice(
    invoice: InvoiceRow,
    lines_by_invoice: &HashMap<i64, Vec<LineRow>>,
    line_taxes: &[TaxLink],
    header_taxes: &[TaxLink],
    terms_by_id: &HashMap<i64, Vec<TermRow>>,
    paid: &HashSet<i64>,
) -> ExportInvoice {
    let lines = lines_by_invoice
        .get(&invoice.id)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let date = india_date(invoice.datetime);
    let header_pct = document_header_percent(lines, line_taxes, header_taxes, invoice.id);
    let mut groups: BTreeMap<Decimal, Slice> = BTreeMap::new();
    let mut hsn = Vec::with_capacity(lines.len());
    if lines.is_empty() {
        groups.insert(
            Decimal::ZERO,
            Slice {
                rate_pct: Decimal::ZERO,
                basic: Decimal::ZERO,
                tax: Decimal::ZERO,
            },
        );
    }
    for line in lines {
        let rate_pct = (line_levied_percent(line.id, line_taxes) + header_pct).round_dp(6);
        let tax = tax_on(line.pre_tax, rate_pct);
        let slice = groups.entry(rate_pct).or_insert(Slice {
            rate_pct,
            basic: Decimal::ZERO,
            tax: Decimal::ZERO,
        });
        slice.basic += line.pre_tax;
        slice.tax += tax;
        let central = tax / Decimal::from(2);
        hsn.push(HsnLine {
            code: line.hsn_code.or(line.sac_code),
            description: line.product_name.clone(),
            uqc: uqc_label(&line.qty_unit),
            quantity: line.quantity,
            total_value: line.pre_tax + tax,
            rate_fraction: rate_pct / Decimal::from(200),
            taxable: line.pre_tax,
            central,
            state: tax - central,
        });
    }
    let slices: Vec<Slice> = groups.into_values().collect();
    let invoice_value = slices
        .iter()
        .map(|slice| slice.basic + slice.tax)
        .sum::<Decimal>();
    let name = if invoice.is_individual {
        invoice.contact_name.trim().to_string()
    } else {
        invoice.company_name.trim().to_string()
    };
    let gstin = if invoice.is_individual {
        String::new()
    } else {
        invoice.gstin.trim().to_string()
    };
    let payment_term = invoice
        .term_id
        .and_then(|id| terms_by_id.get(&id))
        .map(|terms| payment_term_label(date, terms))
        .unwrap_or_default();
    ExportInvoice {
        number: invoice.number,
        date,
        name,
        gstin,
        state: invoice.state.trim().to_string(),
        is_individual: invoice.is_individual,
        receipt: paid.contains(&invoice.id),
        payment_term,
        invoice_value,
        slices,
        hsn,
    }
}

/// Header levied taxes whose id is not already attached to any line.
fn document_header_percent(
    lines: &[LineRow],
    line_taxes: &[TaxLink],
    header_taxes: &[TaxLink],
    invoice_id: i64,
) -> Decimal {
    let line_ids: HashSet<i64> = lines.iter().map(|line| line.id).collect();
    let mut used = HashSet::new();
    for tax in line_taxes {
        if tax.tax_id != 0 && line_ids.contains(&tax.parent_id) {
            used.insert(tax.tax_id);
        }
    }
    header_taxes
        .iter()
        .filter(|tax| {
            tax.parent_id == invoice_id
                && tax.levied
                && tax.tax_id != 0
                && !used.contains(&tax.tax_id)
        })
        .map(|tax| tax.percentage)
        .sum()
}

fn line_levied_percent(line_id: i64, taxes: &[TaxLink]) -> Decimal {
    taxes
        .iter()
        .filter(|tax| tax.parent_id == line_id && tax.levied)
        .map(|tax| tax.percentage)
        .sum()
}

fn tax_on(pre_tax: Decimal, pct: Decimal) -> Decimal {
    if pre_tax.is_zero() || pct.is_zero() {
        Decimal::ZERO
    } else {
        pre_tax * pct / Decimal::from(100)
    }
}

fn payment_term_label(invoice_date: NaiveDate, terms: &[TermRow]) -> String {
    terms
        .iter()
        .map(|term| {
            let days = (term.due_date - invoice_date).num_days();
            if days > 0 {
                format!("{days} DAYS")
            } else {
                "ADVANCE".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn uqc_label(unit: &str) -> String {
    if unit.eq_ignore_ascii_case("number") {
        "NOS-NUMBERS".to_string()
    } else if unit.eq_ignore_ascii_case("kg") {
        "kg".to_string()
    } else {
        unit.to_string()
    }
}

fn india_date(datetime: DateTime<Utc>) -> NaiveDate {
    match FixedOffset::east_opt(19_800) {
        Some(offset) => datetime.with_timezone(&offset).date_naive(),
        None => datetime.date_naive(),
    }
}

fn write_workbook(invoices: &[ExportInvoice]) -> Result<Vec<u8>, String> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold();
    write_invoices(workbook.add_worksheet(), &header, invoices)?;
    write_hsn(workbook.add_worksheet(), &header, invoices)?;
    workbook
        .save_to_buffer()
        .map_err(|err| format!("write workbook: {err}"))
}

fn write_invoices(
    sheet: &mut Worksheet,
    header: &Format,
    invoices: &[ExportInvoice],
) -> Result<(), String> {
    sheet.set_name("invoices").map_err(|err| err.to_string())?;
    for (col, label) in [
        "GSTIN/UIN of Recipient",
        "Receiver Name",
        "Invoice Number",
        "Invoice date",
        "Invoice Value",
        "Place Of Supply",
        "Reverse Charge",
        "Applicable % of Tax Rate",
        "Invoice Type",
        "E-Commerce GSTIN",
        "Rate",
        "Taxable Value",
        "Cess Amount",
        "Payment Term",
    ]
    .iter()
    .enumerate()
    {
        write_header(sheet, header, 0, col, label)?;
    }
    let mut row = 1_u32;
    for invoice in invoices {
        let invoice_type = if invoice.is_individual { "B2C" } else { "B2B" };
        for slice in &invoice.slices {
            write_text(sheet, row, 0, &invoice.gstin)?;
            write_text(sheet, row, 1, &invoice.name)?;
            write_text(sheet, row, 2, &invoice.number)?;
            write_text(sheet, row, 3, &invoice.date.format("%d/%m/%Y").to_string())?;
            write_num(sheet, row, 4, invoice.invoice_value)?;
            write_text(sheet, row, 5, &invoice.state)?;
            write_num(sheet, row, 7, slice.tax)?;
            write_text(sheet, row, 8, invoice_type)?;
            write_num(sheet, row, 10, slice.rate_pct / Decimal::from(100))?;
            write_num(sheet, row, 11, slice.basic)?;
            write_text(sheet, row, 12, receipt_label(invoice.receipt))?;
            write_text(sheet, row, 13, &invoice.payment_term)?;
            row += 1;
        }
    }
    Ok(())
}

fn write_hsn(
    sheet: &mut Worksheet,
    header: &Format,
    invoices: &[ExportInvoice],
) -> Result<(), String> {
    sheet.set_name("HSN").map_err(|err| err.to_string())?;
    write_text(sheet, 0, 0, "Summary For HSN")?;
    for (col, label) in [
        (0_usize, "No. of HSN"),
        (4, "Total Value"),
        (6, "Total Taxable Value"),
        (7, "Total Integrated Tax"),
        (8, "Total Central Tax"),
        (9, "Total State/UT Tax"),
        (10, "Total Cess"),
    ] {
        write_header(sheet, header, 1, col, label)?;
    }
    let lines: Vec<&HsnLine> = invoices.iter().flat_map(|invoice| &invoice.hsn).collect();
    let total_value = lines.iter().map(|line| line.total_value).sum::<Decimal>();
    let taxable = lines.iter().map(|line| line.taxable).sum::<Decimal>();
    let central = lines.iter().map(|line| line.central).sum::<Decimal>();
    let state = lines.iter().map(|line| line.state).sum::<Decimal>();
    let count = i64::try_from(lines.len()).map_err(|err| err.to_string())?;
    write_num(sheet, 2, 0, Decimal::from(count))?;
    write_num(sheet, 2, 4, total_value)?;
    write_num(sheet, 2, 6, taxable)?;
    write_num(sheet, 2, 7, Decimal::ZERO)?;
    write_num(sheet, 2, 8, central)?;
    write_num(sheet, 2, 9, state)?;
    write_num(sheet, 2, 10, Decimal::ZERO)?;
    for (col, label) in [
        "HSN",
        "Description",
        "UQC",
        "Total Quantity",
        "Total Value",
        "Rate",
        "Taxable Value",
        "Integrated Tax Amount",
        "Central Tax Amount",
        "State/UT Tax Amount",
        "Cess Amount",
    ]
    .iter()
    .enumerate()
    {
        write_header(sheet, header, 3, col, label)?;
    }
    for (offset, line) in lines.iter().enumerate() {
        let row = u32::try_from(offset + 4).map_err(|err| err.to_string())?;
        if let Some(code) = line.code {
            write_num(sheet, row, 0, Decimal::from(code))?;
        }
        write_text(sheet, row, 1, &line.description)?;
        write_text(sheet, row, 2, &line.uqc)?;
        write_num(sheet, row, 3, line.quantity)?;
        write_num(sheet, row, 4, line.total_value)?;
        write_num(sheet, row, 5, line.rate_fraction)?;
        write_num(sheet, row, 6, line.taxable)?;
        write_num(sheet, row, 8, line.central)?;
        write_num(sheet, row, 9, line.state)?;
    }
    Ok(())
}

fn receipt_label(paid: bool) -> &'static str {
    if paid { "RECEIPT" } else { "NO" }
}

fn write_header(
    sheet: &mut Worksheet,
    header: &Format,
    row: u32,
    col: usize,
    label: &str,
) -> Result<(), String> {
    let col = u16::try_from(col).map_err(|err| err.to_string())?;
    sheet
        .write_string_with_format(row, col, label, header)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn write_text(sheet: &mut Worksheet, row: u32, col: u16, value: &str) -> Result<(), String> {
    let value = excel_text(value);
    if value.is_empty() {
        return Ok(());
    }
    sheet
        .write_string(row, col, &value)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn write_num(sheet: &mut Worksheet, row: u32, col: u16, value: Decimal) -> Result<(), String> {
    let number = value
        .to_string()
        .parse::<f64>()
        .map_err(|err| err.to_string())?;
    sheet
        .write_number(row, col, number)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn excel_text(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            let u = u32::from(*c);
            u == 0x9 || u == 0xA || u == 0xD || (0x20..0xFFFE).contains(&u)
        })
        .take(EXCEL_MAX_CHARS)
        .collect()
}

fn invoice_row(row: &QueryResult) -> Result<InvoiceRow, String> {
    let id = col_i64(row, 0).ok_or_else(|| "posted invoice is missing an id".to_string())?;
    let datetime = col_text(row, 2);
    let datetime = DateTime::parse_from_rfc3339(&datetime)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|err| format!("posted invoice {id} has no datetime: {err}"))?;
    Ok(InvoiceRow {
        id,
        number: col_text(row, 1),
        datetime,
        term_id: col_i64(row, 3).filter(|id| *id > 0),
        is_individual: is_truthy(&col_text(row, 4)),
        state: col_text(row, 5),
        company_name: col_text(row, 6),
        gstin: col_text(row, 7),
        contact_name: col_text(row, 8),
    })
}

fn line_row(row: &QueryResult) -> Result<LineRow, String> {
    let id = col_i64(row, 0).ok_or_else(|| "invoice line is missing an id".to_string())?;
    Ok(LineRow {
        id,
        invoice_id: col_i64(row, 1).unwrap_or(0),
        product_name: col_text(row, 2),
        hsn_code: col_i64(row, 3),
        sac_code: col_i64(row, 4),
        quantity: col_decimal(row, 5),
        qty_unit: col_text(row, 6),
        pre_tax: col_decimal(row, 7),
    })
}

fn tax_link(row: &QueryResult) -> TaxLink {
    TaxLink {
        parent_id: col_i64(row, 0).unwrap_or(0),
        tax_id: col_i64(row, 1).unwrap_or(0),
        percentage: col_decimal(row, 2),
        levied: col_text(row, 3).eq_ignore_ascii_case("levied"),
    }
}

fn term_row(row: &QueryResult) -> Option<TermRow> {
    let due = parse_date(&col_text(row, 3))?;
    Some(TermRow {
        id: col_i64(row, 0).unwrap_or(0),
        term_id: col_i64(row, 1).unwrap_or(0),
        line_order: i32::try_from(col_i64(row, 2).unwrap_or(0)).unwrap_or(0),
        due_date: due,
    })
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok().or_else(|| {
        raw.get(..10)
            .and_then(|prefix| NaiveDate::parse_from_str(prefix, "%Y-%m-%d").ok())
    })
}

fn col_text(row: &QueryResult, idx: usize) -> String {
    if let Ok(value) = row.try_get_by_index::<Option<String>>(idx) {
        return value.unwrap_or_default();
    }
    if let Ok(Some(value)) = row.try_get_by_index::<Option<i64>>(idx) {
        return value.to_string();
    }
    if let Ok(Some(value)) = row.try_get_by_index::<Option<i32>>(idx) {
        return value.to_string();
    }
    if let Ok(Some(value)) = row.try_get_by_index::<Option<bool>>(idx) {
        return if value {
            "1".to_string()
        } else {
            "0".to_string()
        };
    }
    if let Ok(Some(value)) = row.try_get_by_index::<Option<Decimal>>(idx) {
        return value.normalize().to_string();
    }
    if let Ok(Some(value)) = row.try_get_by_index::<Option<NaiveDate>>(idx) {
        return value.format("%Y-%m-%d").to_string();
    }
    if let Ok(Some(value)) = row.try_get_by_index::<Option<DateTime<Utc>>>(idx) {
        return value.to_rfc3339();
    }
    String::new()
}

fn col_i64(row: &QueryResult, idx: usize) -> Option<i64> {
    let text = col_text(row, idx);
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        text.parse().ok()
    }
}

fn col_decimal(row: &QueryResult, idx: usize) -> Decimal {
    col_text(row, idx).trim().parse().unwrap_or(Decimal::ZERO)
}

fn is_truthy(value: &str) -> bool {
    matches!(value.trim(), "1" | "t" | "true" | "TRUE" | "True")
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
    use zip::ZipArchive;

    use super::build_invoice_workbook;

    #[tokio::test]
    async fn posted_gst_workbook_matches_the_reference_layout() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        for sql in SCHEMA {
            db.execute_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                sql.to_string(),
            ))
            .await
            .expect("schema");
        }
        for sql in FIXTURE {
            db.execute_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                sql.to_string(),
            ))
            .await
            .expect("fixture");
        }
        let bytes = build_invoice_workbook(&db).await.expect("workbook");
        let sheets = read_sheets(&bytes);
        let names: Vec<&str> = sheets.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["invoices", "HSN"]);

        let invoices_sheet = &sheets[0].1;
        assert_eq!(cell(invoices_sheet, 1, "A"), "GSTIN/UIN of Recipient");
        assert_eq!(cell(invoices_sheet, 1, "F"), "Place Of Supply");
        assert_eq!(cell(invoices_sheet, 1, "N"), "Payment Term");
        let b2b = row(invoices_sheet, 2);
        assert_eq!(b2b[0], "27AAAAA0000A1Z5");
        assert_eq!(b2b[1], "Acme");
        assert_eq!(b2b[2], "INV/26-27/2");
        assert_close(&b2b[4], 1390.0);
        assert_eq!(b2b[5], "27");
        assert_eq!(b2b[6], "");
        assert_close(&b2b[7], 10.0);
        assert_eq!(b2b[8], "B2B");
        assert_eq!(b2b[9], "");
        assert_close(&b2b[10], 0.05);
        assert_close(&b2b[11], 200.0);
        assert_eq!(b2b[12], "RECEIPT");
        assert_eq!(b2b[13], "15 DAYS");
        let second_rate = row(invoices_sheet, 3);
        assert_close(&second_rate[4], 1390.0);
        assert_close(&second_rate[11], 1000.0);
        assert_eq!(second_rate[8], "B2B");
        let taxable_sum = parse_num(&b2b[11]) + parse_num(&second_rate[11]);
        let tax_sum = parse_num(&b2b[7]) + parse_num(&second_rate[7]);
        assert!(
            (taxable_sum + tax_sum - parse_num(&b2b[4])).abs() < 1e-6,
            "rate slices should sum to the invoice value"
        );
        let b2c = row(invoices_sheet, 4);
        assert_eq!(b2c[0], "");
        assert_eq!(b2c[5], "");
        assert_eq!(b2c[8], "B2C");
        assert_eq!(b2c[12], "NO");
        assert_eq!(b2c[13], "ADVANCE");

        let hsn = &sheets[1].1;
        assert_eq!(cell(hsn, 1, "A"), "Summary For HSN");
        assert_eq!(cell(hsn, 2, "A"), "No. of HSN");
        assert_eq!(cell(hsn, 2, "E"), "Total Value");
        assert_eq!(cell(hsn, 2, "K"), "Total Cess");
        assert_close(&cell(hsn, 3, "A"), 3.0);
        assert_close(&cell(hsn, 3, "H"), 0.0);
        assert_close(&cell(hsn, 3, "K"), 0.0);
        assert_eq!(cell(hsn, 4, "A"), "HSN");
        assert_eq!(cell(hsn, 4, "C"), "UQC");
        assert_eq!(cell(hsn, 4, "I"), "Central Tax Amount");
        let widget = row(hsn, 5);
        assert_close(&widget[0], 8456.0);
        assert_eq!(widget[1], "Widget");
        assert_eq!(widget[2], "NOS-NUMBERS");
        assert_close(&widget[3], 2.0);
        assert_close(&widget[4], 1180.0);
        assert_close(&widget[5], 0.09);
        assert_close(&widget[6], 1000.0);
        assert_eq!(widget[7], "");
        assert_close(&widget[8], 90.0);
        assert_close(&widget[9], 90.0);
        assert_eq!(widget[10], "");
        let service = row(hsn, 6);
        assert_close(&service[0], 9988.0);
        assert_eq!(service[2], "kg");
        assert_close(&service[5], 0.025);
        assert_close(&service[8], 5.0);
        assert_close(&service[9], 5.0);
    }

    const SCHEMA: &[&str] = &[
        "CREATE TABLE posted_invoices (
            id INTEGER PRIMARY KEY,
            number TEXT,
            datetime TEXT,
            customer_id INTEGER,
            posted_payment_term_id INTEGER
        )",
        "CREATE TABLE customers (
            id INTEGER PRIMARY KEY,
            is_individual INTEGER,
            company_id INTEGER,
            individual_id INTEGER,
            state TEXT
        )",
        "CREATE TABLE crm_companies (id INTEGER PRIMARY KEY, name TEXT, gstin TEXT)",
        "CREATE TABLE crm_contacts (id INTEGER PRIMARY KEY, name TEXT)",
        "CREATE TABLE posted_invoice_lines (
            id INTEGER PRIMARY KEY,
            posted_invoice_id INTEGER,
            product_name TEXT,
            hsn_code INTEGER,
            sac_code INTEGER,
            quantity TEXT,
            qty_unit TEXT,
            pre_tax_amount TEXT
        )",
        "CREATE TABLE taxes (id INTEGER PRIMARY KEY, percentage TEXT, tax_type TEXT)",
        "CREATE TABLE posted_invoice_line_taxes (
            posted_invoice_line_id INTEGER,
            tax_id INTEGER
        )",
        "CREATE TABLE posted_invoice_taxes (posted_invoice_id INTEGER, tax_id INTEGER)",
        "CREATE TABLE posted_payment_term_lines (
            id INTEGER PRIMARY KEY,
            posted_payment_term_id INTEGER,
            line_order INTEGER,
            due_date TEXT
        )",
        "CREATE TABLE paid_invoices (id INTEGER PRIMARY KEY, posted_invoice_id INTEGER)",
        "CREATE TABLE partially_paid_invoices (id INTEGER PRIMARY KEY, posted_invoice_id INTEGER)",
    ];

    const FIXTURE: &[&str] = &[
        "INSERT INTO crm_companies (id, name, gstin) VALUES (4, 'Acme', '27AAAAA0000A1Z5')",
        "INSERT INTO crm_contacts (id, name) VALUES (9, 'Ada')",
        "INSERT INTO customers (id, is_individual, company_id, state) VALUES (1, 0, 4, '27')",
        "INSERT INTO customers (id, is_individual, individual_id) VALUES (2, 1, 9)",
        "INSERT INTO posted_invoices (id, number, datetime, customer_id, posted_payment_term_id)
         VALUES (1, 'INV/26-27/2', '2026-10-07T18:30:00+00:00', 1, 7)",
        "INSERT INTO posted_invoices (id, number, datetime, customer_id, posted_payment_term_id)
         VALUES (2, 'INV/26-27/3', '2026-10-08T18:30:00+00:00', 2, 8)",
        "INSERT INTO posted_invoice_lines
            (id, posted_invoice_id, product_name, hsn_code, quantity, qty_unit, pre_tax_amount)
         VALUES (10, 1, 'Widget', 8456, '2', 'Number', '1000')",
        "INSERT INTO posted_invoice_lines
            (id, posted_invoice_id, product_name, sac_code, quantity, qty_unit, pre_tax_amount)
         VALUES (11, 1, 'Service', 9988, '3', 'kg', '200')",
        "INSERT INTO posted_invoice_lines
            (id, posted_invoice_id, product_name, hsn_code, quantity, qty_unit, pre_tax_amount)
         VALUES (12, 2, 'Cut', 7209, '1', 'kg', '50')",
        "INSERT INTO taxes (id, percentage, tax_type) VALUES
            (1, '17', 'levied'),
            (2, '4', 'levied'),
            (3, '1', 'levied'),
            (4, '10', 'withholding')",
        "INSERT INTO posted_invoice_line_taxes (posted_invoice_line_id, tax_id) VALUES
            (10, 1), (10, 4), (11, 2)",
        "INSERT INTO posted_invoice_taxes (posted_invoice_id, tax_id) VALUES (1, 3)",
        "INSERT INTO posted_payment_term_lines (id, posted_payment_term_id, line_order, due_date)
         VALUES (1, 7, 1, '2026-10-23'), (2, 8, 1, '2026-10-09')",
        "INSERT INTO paid_invoices (id, posted_invoice_id) VALUES (1, 1)",
    ];

    fn assert_close(raw: &str, expected: f64) {
        let actual = parse_num(raw);
        assert!(
            (actual - expected).abs() < 1e-6,
            "{raw} parsed as {actual}, expected {expected}"
        );
    }

    fn parse_num(raw: &str) -> f64 {
        raw.parse().expect("number")
    }

    fn row(sheet: &SheetGrid, row_num: u32) -> Vec<String> {
        let mut cells = sheet.get(&row_num).cloned().unwrap_or_default();
        cells.sort_by(|left, right| left.0.cmp(&right.0));
        let mut out = vec![String::new(); 14];
        for (col, value) in cells {
            let Some(idx) = col_index(&col) else {
                continue;
            };
            if let Some(slot) = out.get_mut(idx) {
                *slot = value;
            }
        }
        out
    }

    fn cell(sheet: &SheetGrid, row_num: u32, col: &str) -> String {
        sheet
            .get(&row_num)
            .and_then(|cells| {
                cells
                    .iter()
                    .find(|(name, _)| name == col)
                    .map(|(_, value)| value.clone())
            })
            .unwrap_or_default()
    }

    type SheetGrid = std::collections::BTreeMap<u32, Vec<(String, String)>>;

    fn read_sheets(bytes: &[u8]) -> Vec<(String, SheetGrid)> {
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip");
        let shared = zip_string(&mut zip, "xl/sharedStrings.xml");
        let strings = shared_strings(&shared);
        let workbook = zip_string(&mut zip, "xl/workbook.xml");
        let rels = zip_string(&mut zip, "xl/_rels/workbook.xml.rels");
        let mut sheets = Vec::new();
        for (name, rid) in sheet_refs(&workbook) {
            let target = rel_target(&rels, &rid);
            let path = if target.starts_with("xl/") {
                target
            } else if target.starts_with('/') {
                target.trim_start_matches('/').to_string()
            } else {
                format!("xl/{target}")
            };
            let xml = zip_string(&mut zip, &path);
            sheets.push((name, grid(&xml, &strings)));
        }
        sheets
    }

    fn zip_string(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> String {
        let mut file = zip.by_name(name).expect(name);
        let mut text = String::new();
        std::io::Read::read_to_string(&mut file, &mut text).expect("read");
        text
    }

    fn shared_strings(xml: &str) -> Vec<String> {
        let mut out = Vec::new();
        for item in xml.split("<si>").skip(1) {
            let mut text = String::new();
            let mut rest = item;
            while let Some(start) = rest.find("<t") {
                let after = rest.get(start..).unwrap_or("");
                let Some(open) = after.find('>') else {
                    break;
                };
                let body = after.get(open + 1..).unwrap_or("");
                let Some(end) = body.find("</t>") else {
                    break;
                };
                text.push_str(body.get(..end).unwrap_or(""));
                rest = body.get(end + 4..).unwrap_or("");
            }
            out.push(unescape(&text));
        }
        out
    }

    fn sheet_refs(workbook: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for chunk in workbook.split("<sheet ").skip(1) {
            let head = chunk.split('>').next().unwrap_or("");
            let name = attr(head, "name");
            let rid = attr(head, "r:id");
            if !name.is_empty() {
                out.push((unescape(&name), rid));
            }
        }
        out
    }

    fn rel_target(rels: &str, id: &str) -> String {
        for chunk in rels.split("<Relationship ").skip(1) {
            let head = chunk.split('>').next().unwrap_or("");
            if attr(head, "Id") == id {
                return attr(head, "Target");
            }
        }
        String::new()
    }

    fn grid(xml: &str, strings: &[String]) -> SheetGrid {
        let mut rows = SheetGrid::new();
        for chunk in xml.split("<c ").skip(1) {
            let head = chunk.split('>').next().unwrap_or("");
            let reference = attr(head, "r");
            let (col, row_num) = split_ref(&reference);
            let raw = tag_text(chunk, "v");
            let value = if head.contains("t=\"s\"") {
                raw.parse::<usize>()
                    .ok()
                    .and_then(|idx| strings.get(idx))
                    .cloned()
                    .unwrap_or_default()
            } else {
                unescape(&raw)
            };
            if !col.is_empty() {
                rows.entry(row_num).or_default().push((col, value));
            }
        }
        rows
    }

    fn split_ref(reference: &str) -> (String, u32) {
        let col: String = reference
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let row = reference
            .chars()
            .skip_while(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        (col, row)
    }

    fn col_index(col: &str) -> Option<usize> {
        let mut idx = 0_usize;
        for ch in col.chars() {
            let value = ch.to_digit(36)? - 10;
            idx = idx * 26 + usize::try_from(value).unwrap_or(0) + 1;
        }
        idx.checked_sub(1)
    }

    fn attr(tag: &str, name: &str) -> String {
        let needle = format!("{name}=\"");
        let Some(start) = tag.find(&needle) else {
            return String::new();
        };
        let rest = tag.get(start + needle.len()..).unwrap_or("");
        rest.split('"').next().unwrap_or("").to_string()
    }

    fn tag_text(chunk: &str, tag: &str) -> String {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let Some(start) = chunk.find(&open) else {
            return String::new();
        };
        let rest = chunk.get(start + open.len()..).unwrap_or("");
        rest.split(&close).next().unwrap_or("").to_string()
    }

    fn unescape(value: &str) -> String {
        value
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
    }
}
