//! Customer statement workbook: posted invoices and payments, plus balances.

use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use lariv_plugin_finance_accounts::scope::load_default_currency_format;

const EXCEL_MAX_CHARS: usize = 32_767;

const COL_SR: u16 = 0;
const COL_NUMBER: u16 = 1;
const COL_DATE: u16 = 2;
const COL_TYPE: u16 = 3;
const COL_REMARKS: u16 = 4;
const COL_AMOUNT: u16 = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Invoice,
    Payment,
}

impl EntryKind {
    fn label(self) -> &'static str {
        match self {
            Self::Invoice => "Invoice",
            Self::Payment => "Payment",
        }
    }

    fn sort_key(self) -> u8 {
        match self {
            Self::Invoice => 0,
            Self::Payment => 1,
        }
    }
}

struct Entry {
    number: String,
    datetime: DateTime<Utc>,
    kind: EntryKind,
    remarks: String,
    amount: Decimal,
}

struct StatementTotals {
    invoices: Decimal,
    payments: Decimal,
    net: Decimal,
}

struct CustomerHeader {
    name: String,
    kind: &'static str,
    address: String,
    phone: String,
    email: String,
    gstin: String,
    pan: String,
    cin: String,
    website: String,
}

/// Posted (non-cancelled) invoices and payments for one customer.
pub async fn build_customer_statement(
    db: &DatabaseConnection,
    customer_id: i64,
    timezone: &str,
) -> Result<Vec<u8>, String> {
    let customer = load_customer_header(db, customer_id).await?;
    let mut entries = load_invoices(db, customer_id).await?;
    entries.extend(load_payments(db, customer_id).await?);
    sort_entries(&mut entries);
    let minor_unit = load_default_currency_format(db).await.minor_unit;
    write_workbook(&customer, &entries, timezone, Utc::now(), minor_unit)
}

async fn load_customer_header<C: ConnectionTrait>(
    db: &C,
    customer_id: i64,
) -> Result<CustomerHeader, String> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT c.is_individual AS is_individual, \
                    CASE WHEN c.is_individual \
                      THEN COALESCE(NULLIF(ct.name, ''), '#' || CAST(c.id AS TEXT)) \
                      ELSE COALESCE(NULLIF(co.name, ''), '#' || CAST(c.id AS TEXT)) \
                    END AS name, \
                    co.address_line_1 AS address_line_1, \
                    co.address_line_2 AS address_line_2, \
                    co.city AS city, \
                    co.pincode AS pincode, \
                    co.state AS state, \
                    CASE WHEN c.is_individual THEN ct.phone ELSE co.phone END AS phone, \
                    CASE WHEN c.is_individual THEN ct.email ELSE co.email END AS email, \
                    co.gstin AS gstin, \
                    co.pan AS pan, \
                    co.cin AS cin, \
                    co.website AS website \
             FROM customers c \
             LEFT JOIN crm_companies co ON co.id = c.company_id \
             LEFT JOIN crm_contacts ct ON ct.id = c.individual_id \
             WHERE c.id = $1",
            vec![customer_id.into()],
        ))
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "customer not found".to_string())?;
    let is_individual: bool = row
        .try_get("", "is_individual")
        .map_err(|err| err.to_string())?;
    let address = [
        optional_text(&row, "address_line_1")?,
        optional_text(&row, "address_line_2")?,
        optional_text(&row, "city")?,
        optional_text(&row, "pincode")?,
        optional_text(&row, "state")?,
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(", ");
    Ok(CustomerHeader {
        name: optional_text(&row, "name")?,
        kind: if is_individual { "Contact" } else { "Company" },
        address,
        phone: optional_text(&row, "phone")?,
        email: optional_text(&row, "email")?,
        gstin: optional_text(&row, "gstin")?,
        pan: optional_text(&row, "pan")?,
        cin: optional_text(&row, "cin")?,
        website: optional_text(&row, "website")?,
    })
}

fn optional_text(row: &sea_orm::QueryResult, column: &str) -> Result<String, String> {
    let value: Option<String> = row.try_get("", column).map_err(|err| err.to_string())?;
    Ok(value.unwrap_or_default().trim().to_string())
}

fn customer_detail_rows(customer: &CustomerHeader, generated_at: &str) -> Vec<(String, String)> {
    let mut rows = vec![
        ("Customer".to_string(), customer.name.clone()),
        ("Kind".to_string(), customer.kind.to_string()),
    ];
    push_detail(&mut rows, "Address", &customer.address);
    push_detail(&mut rows, "Phone", &customer.phone);
    push_detail(&mut rows, "Email", &customer.email);
    push_detail(&mut rows, "GSTIN", &customer.gstin);
    push_detail(&mut rows, "PAN", &customer.pan);
    push_detail(&mut rows, "CIN", &customer.cin);
    push_detail(&mut rows, "Website", &customer.website);
    rows.push(("Generated at".to_string(), generated_at.to_string()));
    rows
}

fn push_detail(rows: &mut Vec<(String, String)>, label: &str, value: &str) {
    if !value.is_empty() {
        rows.push((label.to_string(), value.to_string()));
    }
}

async fn load_invoices<C: ConnectionTrait>(db: &C, customer_id: i64) -> Result<Vec<Entry>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT i.number AS item_number, \
                    i.datetime AS datetime, \
                    COALESCE(i.remarks, '') AS remarks, \
                    COALESCE(SUM(l.amount), 0) AS amount \
             FROM posted_invoices i \
             LEFT JOIN posted_payment_term_lines l \
               ON l.posted_payment_term_id = i.posted_payment_term_id \
             WHERE i.customer_id = $1 \
               AND NOT EXISTS ( \
                 SELECT 1 FROM cancelled_invoices c WHERE c.posted_invoice_id = i.id \
               ) \
             GROUP BY i.id",
            vec![customer_id.into()],
        ))
        .await
        .map_err(|err| err.to_string())?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        entries.push(Entry {
            number: row
                .try_get("", "item_number")
                .map_err(|err| err.to_string())?,
            datetime: row.try_get("", "datetime").map_err(|err| err.to_string())?,
            kind: EntryKind::Invoice,
            remarks: row.try_get("", "remarks").map_err(|err| err.to_string())?,
            amount: row.try_get("", "amount").map_err(|err| err.to_string())?,
        });
    }
    Ok(entries)
}

async fn load_payments<C: ConnectionTrait>(db: &C, customer_id: i64) -> Result<Vec<Entry>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT number, datetime, amount FROM payments WHERE customer_id = $1",
            vec![customer_id.into()],
        ))
        .await
        .map_err(|err| err.to_string())?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        entries.push(Entry {
            number: row.try_get("", "number").map_err(|err| err.to_string())?,
            datetime: row.try_get("", "datetime").map_err(|err| err.to_string())?,
            kind: EntryKind::Payment,
            remarks: String::new(),
            amount: row.try_get("", "amount").map_err(|err| err.to_string())?,
        });
    }
    Ok(entries)
}

fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        a.datetime
            .cmp(&b.datetime)
            .then_with(|| a.kind.sort_key().cmp(&b.kind.sort_key()))
            .then_with(|| a.number.cmp(&b.number))
    });
}

fn statement_totals(entries: &[Entry]) -> Result<StatementTotals, String> {
    let mut invoices = Decimal::ZERO;
    let mut payments = Decimal::ZERO;
    for entry in entries {
        match entry.kind {
            EntryKind::Invoice => {
                invoices = invoices
                    .checked_add(entry.amount)
                    .ok_or_else(|| "invoice total overflowed".to_string())?;
            }
            EntryKind::Payment => {
                payments = payments
                    .checked_add(entry.amount)
                    .ok_or_else(|| "payment total overflowed".to_string())?;
            }
        }
    }
    let net = invoices
        .checked_sub(payments)
        .ok_or_else(|| "net balance overflowed".to_string())?;
    Ok(StatementTotals {
        invoices,
        payments,
        net,
    })
}

fn amount_num_format(minor_unit: i32) -> String {
    let places = minor_unit.clamp(0, 6);
    if places == 0 {
        return "#,##0".to_string();
    }
    let zeros = "0".repeat(usize::try_from(places).unwrap_or(2));
    format!("#,##0.{zeros}")
}

fn write_workbook(
    customer: &CustomerHeader,
    entries: &[Entry],
    timezone: &str,
    generated_at: DateTime<Utc>,
    minor_unit: i32,
) -> Result<Vec<u8>, String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name("Statement")
        .map_err(|err| err.to_string())?;
    // Cells stay locked by default. Excel refuses edits until the sheet is unprotected.
    worksheet.protect();

    let title_format = Format::new().set_bold().set_font_size(16);
    let label_format = Format::new().set_bold();
    let header = Format::new().set_bold();
    let date_format = Format::new().set_num_format("dd/mm/yyyy");
    let amount_format = Format::new().set_num_format(amount_num_format(minor_unit));
    let summary_label = Format::new().set_bold();
    let summary_amount = Format::new()
        .set_bold()
        .set_num_format(amount_num_format(minor_unit));

    let generated = lariv_core::datetime::DatetimeLabel::display(generated_at, timezone);
    let details = customer_detail_rows(customer, generated.as_str());
    worksheet
        .merge_range(0, COL_SR, 0, COL_AMOUNT, "Statement", &title_format)
        .map_err(|err| err.to_string())?;
    for (offset, (label, value)) in details.iter().enumerate() {
        let row = u32::try_from(offset.saturating_add(1)).map_err(|err| err.to_string())?;
        worksheet
            .write_string_with_format(row, COL_SR, label, &label_format)
            .map_err(|err| err.to_string())?;
        worksheet
            .write_string(row, COL_NUMBER, sanitize_xlsx_string(value))
            .map_err(|err| err.to_string())?;
    }
    let column_row =
        u32::try_from(details.len().saturating_add(2)).map_err(|err| err.to_string())?;
    for (col, title) in [
        (COL_SR, "Sr. No."),
        (COL_NUMBER, "Item Number"),
        (COL_DATE, "Date"),
        (COL_TYPE, "Type"),
        (COL_REMARKS, "Remarks"),
        (COL_AMOUNT, "Amount"),
    ] {
        worksheet
            .write_string_with_format(column_row, col, title, &header)
            .map_err(|err| err.to_string())?;
    }

    let first_data = column_row.saturating_add(1);
    for (offset, entry) in entries.iter().enumerate() {
        let delta = u32::try_from(offset).map_err(|err| err.to_string())?;
        write_entry(
            worksheet,
            first_data.saturating_add(delta),
            delta.saturating_add(1),
            entry,
            timezone,
            &date_format,
            &amount_format,
        )?;
    }

    let totals = statement_totals(entries)?;
    let data_rows = u32::try_from(entries.len()).map_err(|err| err.to_string())?;
    let first_total = first_data.saturating_add(data_rows);
    write_total(
        worksheet,
        first_total,
        "Total Invoices",
        totals.invoices,
        &summary_label,
        &summary_amount,
    )?;
    write_total(
        worksheet,
        first_total.saturating_add(1),
        "Total Payments",
        totals.payments,
        &summary_label,
        &summary_amount,
    )?;
    write_total(
        worksheet,
        first_total.saturating_add(2),
        "Net Balance",
        totals.net,
        &summary_label,
        &summary_amount,
    )?;

    worksheet
        .set_column_width(COL_SR, 18)
        .map_err(|err| err.to_string())?;
    worksheet
        .set_column_width(COL_NUMBER, 42)
        .map_err(|err| err.to_string())?;
    worksheet
        .set_column_width(COL_DATE, 14)
        .map_err(|err| err.to_string())?;
    worksheet
        .set_column_width(COL_TYPE, 14)
        .map_err(|err| err.to_string())?;
    worksheet
        .set_column_width(COL_REMARKS, 40)
        .map_err(|err| err.to_string())?;
    worksheet
        .set_column_width(COL_AMOUNT, 16)
        .map_err(|err| err.to_string())?;

    workbook.save_to_buffer().map_err(|err| err.to_string())
}

fn write_entry(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    sr: u32,
    entry: &Entry,
    timezone: &str,
    date_format: &Format,
    amount_format: &Format,
) -> Result<(), String> {
    worksheet
        .write_number(row, COL_SR, sr)
        .map_err(|err| err.to_string())?;
    worksheet
        .write_string(row, COL_NUMBER, sanitize_xlsx_string(&entry.number))
        .map_err(|err| err.to_string())?;
    let local = entry
        .datetime
        .with_timezone(&lariv_core::datetime::parse_timezone(timezone));
    let excel_date = ExcelDateTime::from_ymd(
        u16::try_from(local.year()).map_err(|err| err.to_string())?,
        u8::try_from(local.month()).map_err(|err| err.to_string())?,
        u8::try_from(local.day()).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    worksheet
        .write_datetime_with_format(row, COL_DATE, &excel_date, date_format)
        .map_err(|err| err.to_string())?;
    worksheet
        .write_string(row, COL_TYPE, entry.kind.label())
        .map_err(|err| err.to_string())?;
    worksheet
        .write_string(row, COL_REMARKS, sanitize_xlsx_string(&entry.remarks))
        .map_err(|err| err.to_string())?;
    write_amount(worksheet, row, entry.amount, amount_format)
}

fn write_total(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    label: &str,
    amount: Decimal,
    label_format: &Format,
    amount_format: &Format,
) -> Result<(), String> {
    worksheet
        .write_string_with_format(row, COL_SR, label, label_format)
        .map_err(|err| err.to_string())?;
    write_amount(worksheet, row, amount, amount_format)
}

fn write_amount(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    amount: Decimal,
    amount_format: &Format,
) -> Result<(), String> {
    let number = amount
        .to_f64()
        .ok_or_else(|| "amount is not a finite number".to_string())?;
    worksheet
        .write_number_with_format(row, COL_AMOUNT, number, amount_format)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn sanitize_xlsx_string(s: &str) -> String {
    let filtered: String = s
        .chars()
        .filter(|c| {
            let u = u32::from(*c);
            u == 0x9 || u == 0xA || u == 0xD || (0x20..0xFFFE).contains(&u)
        })
        .collect();
    filtered.chars().take(EXCEL_MAX_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        CustomerHeader, Entry, EntryKind, customer_detail_rows, sort_entries, statement_totals,
    };
    use chrono::{TimeZone, Utc};
    use rust_decimal::Decimal;

    #[test]
    fn statement_totals_subtract_payments_from_invoices() {
        let entries = [
            entry_amount("INV-1", EntryKind::Invoice, "100.50"),
            entry_amount("PAY-1", EntryKind::Payment, "40.25"),
            entry_amount("INV-2", EntryKind::Invoice, "10"),
        ];
        let totals = statement_totals(&entries).expect("totals");
        assert_eq!(totals.invoices, dec("110.50"));
        assert_eq!(totals.payments, dec("40.25"));
        assert_eq!(totals.net, dec("70.25"));
    }

    #[test]
    fn empty_statement_totals_are_zero() {
        let totals = statement_totals(&[]).expect("totals");
        assert_eq!(totals.invoices, Decimal::ZERO);
        assert_eq!(totals.payments, Decimal::ZERO);
        assert_eq!(totals.net, Decimal::ZERO);
    }

    #[test]
    fn header_lists_customer_details_and_generated_at() {
        let customer = CustomerHeader {
            name: "Acme".into(),
            kind: "Company",
            address: "1 Road, Pune".into(),
            phone: String::new(),
            email: "a@acme.test".into(),
            gstin: "27AAAAA0000A1Z5".into(),
            pan: String::new(),
            cin: String::new(),
            website: String::new(),
        };
        let rows = customer_detail_rows(&customer, "05/11/2026 15:11:00");
        let labels: Vec<&str> = rows.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Customer",
                "Kind",
                "Address",
                "Email",
                "GSTIN",
                "Generated at"
            ]
        );
        assert_eq!(rows.first().map(|(_, value)| value.as_str()), Some("Acme"));
        assert_eq!(
            rows.last().map(|(_, value)| value.as_str()),
            Some("05/11/2026 15:11:00")
        );
    }

    #[test]
    fn entries_sort_by_date_then_invoice_before_payment() {
        let invoice_later = entry("INV-2", 2026, 5, 2, EntryKind::Invoice);
        let payment_same_day = entry("4", 2026, 5, 1, EntryKind::Payment);
        let invoice_same_day = entry("INV-1", 2026, 5, 1, EntryKind::Invoice);
        let mut entries = vec![invoice_later, payment_same_day, invoice_same_day];
        sort_entries(&mut entries);
        let numbers: Vec<&str> = entries.iter().map(|entry| entry.number.as_str()).collect();
        assert_eq!(numbers, ["INV-1", "4", "INV-2"]);
    }

    fn entry(number: &str, year: i32, month: u32, day: u32, kind: EntryKind) -> Entry {
        Entry {
            number: number.to_string(),
            datetime: Utc
                .with_ymd_and_hms(year, month, day, 0, 0, 0)
                .single()
                .expect("valid utc"),
            kind,
            remarks: String::new(),
            amount: Decimal::ZERO,
        }
    }

    fn entry_amount(number: &str, kind: EntryKind, amount: &str) -> Entry {
        let mut row = entry(number, 2026, 5, 1, kind);
        row.amount = dec(amount);
        row
    }

    fn dec(amount: &str) -> Decimal {
        amount.parse().expect("decimal")
    }
}
