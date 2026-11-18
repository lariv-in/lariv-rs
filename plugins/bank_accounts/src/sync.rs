//! Fetch and upsert the Razorpay IFSC branch directory.

use std::collections::HashMap;
use std::io::Cursor;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, Statement, Value,
};
use serde::Deserialize;

use crate::entities::bank_directory_sync::{self, Entity as BankDirectorySyncEntity};

const GITHUB_LATEST: &str = "https://api.github.com/repos/razorpay/ifsc/releases/latest";
const USER_AGENT: &str = "lariv-bank-accounts/0.1";
const UPSERT_BATCH: usize = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankMeta {
    pub ach_credit: bool,
    pub ach_debit: bool,
    pub apbs: bool,
    pub nach_debit: bool,
    pub bank_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankBranchRow {
    pub bank_name: Option<String>,
    pub bank_code: String,
    pub ifsc: String,
    pub branch_name: Option<String>,
    pub center: Option<String>,
    pub district: Option<String>,
    pub state: Option<String>,
    pub address: Option<String>,
    pub contact: Option<String>,
    pub imps: bool,
    pub rtgs: bool,
    pub city: Option<String>,
    pub iso3166: Option<String>,
    pub neft: bool,
    pub micr: Option<String>,
    pub upi: bool,
    pub swift: Option<String>,
    pub ach_credit: bool,
    pub ach_debit: bool,
    pub apbs: bool,
    pub nach_debit: bool,
    pub bank_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
}

#[derive(Debug, Deserialize)]
struct BanksJsonEntry {
    #[serde(default)]
    ach_credit: bool,
    #[serde(default)]
    ach_debit: bool,
    #[serde(default)]
    apbs: bool,
    #[serde(default)]
    nach_debit: bool,
    #[serde(default, rename = "type")]
    bank_type: Option<String>,
}

fn opt_cell(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_bool_cell(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "y"
    )
}

/// Resolve bank code from IFSC, honouring Razorpay sublets.
pub fn bank_code_for_ifsc(ifsc: &str, sublets: &HashMap<String, String>) -> String {
    if let Some(code) = sublets.get(ifsc) {
        return code.clone();
    }
    ifsc.chars().take(4).collect()
}

/// Join one CSV row with banks.json / sublet.json metadata.
pub fn join_branch_row(
    csv: &HashMap<String, String>,
    banks: &HashMap<String, BankMeta>,
    sublets: &HashMap<String, String>,
) -> Option<BankBranchRow> {
    let ifsc = opt_cell(csv.get("IFSC").map(String::as_str).unwrap_or(""))?;
    let bank_code = bank_code_for_ifsc(&ifsc, sublets);
    let meta = banks.get(&bank_code);
    Some(BankBranchRow {
        bank_name: opt_cell(csv.get("BANK").map(String::as_str).unwrap_or("")),
        bank_code,
        ifsc,
        branch_name: opt_cell(csv.get("BRANCH").map(String::as_str).unwrap_or("")),
        center: opt_cell(csv.get("CENTRE").map(String::as_str).unwrap_or("")),
        district: opt_cell(csv.get("DISTRICT").map(String::as_str).unwrap_or("")),
        state: opt_cell(csv.get("STATE").map(String::as_str).unwrap_or("")),
        address: opt_cell(csv.get("ADDRESS").map(String::as_str).unwrap_or("")),
        contact: opt_cell(csv.get("CONTACT").map(String::as_str).unwrap_or("")),
        imps: parse_bool_cell(csv.get("IMPS").map(String::as_str).unwrap_or("")),
        rtgs: parse_bool_cell(csv.get("RTGS").map(String::as_str).unwrap_or("")),
        city: opt_cell(csv.get("CITY").map(String::as_str).unwrap_or("")),
        iso3166: opt_cell(csv.get("ISO3166").map(String::as_str).unwrap_or("")),
        neft: parse_bool_cell(csv.get("NEFT").map(String::as_str).unwrap_or("")),
        micr: opt_cell(csv.get("MICR").map(String::as_str).unwrap_or("")),
        upi: parse_bool_cell(csv.get("UPI").map(String::as_str).unwrap_or("")),
        swift: opt_cell(csv.get("SWIFT").map(String::as_str).unwrap_or("")),
        ach_credit: meta.map(|m| m.ach_credit).unwrap_or(false),
        ach_debit: meta.map(|m| m.ach_debit).unwrap_or(false),
        apbs: meta.map(|m| m.apbs).unwrap_or(false),
        nach_debit: meta.map(|m| m.nach_debit).unwrap_or(false),
        bank_type: meta.and_then(|m| m.bank_type.clone()),
    })
}

pub fn parse_banks_json(bytes: &[u8]) -> Result<HashMap<String, BankMeta>, String> {
    let raw: HashMap<String, BanksJsonEntry> =
        serde_json::from_slice(bytes).map_err(|e| format!("banks.json: {e}"))?;
    Ok(raw
        .into_iter()
        .map(|(code, entry)| {
            (
                code,
                BankMeta {
                    ach_credit: entry.ach_credit,
                    ach_debit: entry.ach_debit,
                    apbs: entry.apbs,
                    nach_debit: entry.nach_debit,
                    bank_type: entry.bank_type.and_then(|t| {
                        let t = t.trim();
                        if t.is_empty() {
                            None
                        } else {
                            Some(t.to_string())
                        }
                    }),
                },
            )
        })
        .collect())
}

pub fn parse_sublet_json(bytes: &[u8]) -> Result<HashMap<String, String>, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("sublet.json: {e}"))
}

pub fn parse_ifsc_csv(
    bytes: &[u8],
    banks: &HashMap<String, BankMeta>,
    sublets: &HashMap<String, String>,
) -> Result<Vec<BankBranchRow>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(Cursor::new(bytes));
    let headers = reader
        .headers()
        .map_err(|e| format!("IFSC.csv headers: {e}"))?
        .clone();
    let mut rows = Vec::new();
    for (idx, record) in reader.records().enumerate() {
        let record = record.map_err(|e| format!("IFSC.csv row {}: {e}", idx + 2))?;
        let mut map = HashMap::new();
        for (header, value) in headers.iter().zip(record.iter()) {
            map.insert(header.to_string(), value.to_string());
        }
        if let Some(row) = join_branch_row(&map, banks, sublets) {
            rows.push(row);
        }
    }
    Ok(rows)
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn placeholder(backend: DatabaseBackend, n: usize) -> String {
    match backend {
        DatabaseBackend::Postgres => format!("${n}"),
        _ => "?".to_string(),
    }
}

async fn upsert_batch<C: ConnectionTrait>(
    db: &C,
    rows: &[BankBranchRow],
    now: DateTime<Utc>,
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let backend = db.get_database_backend();
    let columns = [
        "created_at",
        "updated_at",
        "bank_name",
        "bank_code",
        "ifsc",
        "branch_name",
        "center",
        "district",
        "state",
        "address",
        "contact",
        "imps",
        "rtgs",
        "city",
        "iso3166",
        "neft",
        "micr",
        "upi",
        "swift",
        "ach_credit",
        "ach_debit",
        "apbs",
        "nach_debit",
        "bank_type",
    ];
    let col_sql = columns
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let mut values_sql = Vec::with_capacity(rows.len());
    let mut params: Vec<Value> = Vec::with_capacity(rows.len() * columns.len());
    let mut bind = 1usize;
    for row in rows {
        let mut placeholders = Vec::with_capacity(columns.len());
        for _ in 0..columns.len() {
            placeholders.push(placeholder(backend, bind));
            bind = bind.saturating_add(1);
        }
        values_sql.push(format!("({})", placeholders.join(", ")));
        params.push(Value::from(now));
        params.push(Value::from(now));
        params.push(Value::from(row.bank_name.clone()));
        params.push(Value::from(row.bank_code.clone()));
        params.push(Value::from(row.ifsc.clone()));
        params.push(Value::from(row.branch_name.clone()));
        params.push(Value::from(row.center.clone()));
        params.push(Value::from(row.district.clone()));
        params.push(Value::from(row.state.clone()));
        params.push(Value::from(row.address.clone()));
        params.push(Value::from(row.contact.clone()));
        params.push(Value::from(row.imps));
        params.push(Value::from(row.rtgs));
        params.push(Value::from(row.city.clone()));
        params.push(Value::from(row.iso3166.clone()));
        params.push(Value::from(row.neft));
        params.push(Value::from(row.micr.clone()));
        params.push(Value::from(row.upi));
        params.push(Value::from(row.swift.clone()));
        params.push(Value::from(row.ach_credit));
        params.push(Value::from(row.ach_debit));
        params.push(Value::from(row.apbs));
        params.push(Value::from(row.nach_debit));
        params.push(Value::from(row.bank_type.clone()));
    }
    let updates = [
        "updated_at",
        "bank_name",
        "bank_code",
        "branch_name",
        "center",
        "district",
        "state",
        "address",
        "contact",
        "imps",
        "rtgs",
        "city",
        "iso3166",
        "neft",
        "micr",
        "upi",
        "swift",
        "ach_credit",
        "ach_debit",
        "apbs",
        "nach_debit",
        "bank_type",
    ]
    .iter()
    .map(|c| format!("{} = EXCLUDED.{}", quote_ident(c), quote_ident(c)))
    .collect::<Vec<_>>()
    .join(", ");
    let sql = format!(
        "INSERT INTO {} ({}) VALUES {} ON CONFLICT ({}) DO UPDATE SET {}",
        quote_ident("banks"),
        col_sql,
        values_sql.join(", "),
        quote_ident("ifsc"),
        updates
    );
    let stmt = Statement::from_sql_and_values(backend, sql, params);
    db.execute_raw(stmt)
        .await
        .map_err(|e| format!("upsert banks: {e}"))?;
    Ok(())
}

pub async fn upsert_banks<C: ConnectionTrait>(
    db: &C,
    rows: &[BankBranchRow],
) -> Result<(), String> {
    let now = Utc::now();
    for chunk in rows.chunks(UPSERT_BATCH) {
        upsert_batch(db, chunk, now).await?;
    }
    Ok(())
}

pub async fn last_success_at(db: &DatabaseConnection) -> Option<DateTime<Utc>> {
    lariv_core::web::opt_or_log(
        BankDirectorySyncEntity::find_by_id(1).one(db).await,
        "load bank directory sync",
    )
    .map(|row| row.last_success_at)
}

pub async fn mark_success(db: &DatabaseConnection, at: DateTime<Utc>) -> Result<(), String> {
    match BankDirectorySyncEntity::find_by_id(1)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(existing) => {
            let mut am: bank_directory_sync::ActiveModel = existing.into();
            am.last_success_at = Set(at);
            am.update(db).await.map_err(|e| e.to_string())?;
        }
        None => {
            bank_directory_sync::ActiveModel {
                id: Set(1),
                last_success_at: Set(at),
            }
            .insert(db)
            .await
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

async fn http_get_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("GET {url}: HTTP {}", response.status()));
    }
    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("GET {url} body: {e}"))
}

pub async fn resolve_latest_tag(client: &reqwest::Client) -> Result<String, String> {
    let bytes = http_get_bytes(client, GITHUB_LATEST).await?;
    let release: GithubRelease =
        serde_json::from_slice(&bytes).map_err(|e| format!("release json: {e}"))?;
    if release.tag_name.trim().is_empty() {
        return Err("release tag_name empty".into());
    }
    Ok(release.tag_name)
}

fn asset_url(tag: &str, name: &str) -> String {
    format!("https://github.com/razorpay/ifsc/releases/download/{tag}/{name}")
}

/// Download, join, and upsert the latest Razorpay IFSC release.
pub async fn sync_directory(db: &DatabaseConnection) -> Result<usize, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    let tag = resolve_latest_tag(&client).await?;
    tracing::info!(target: "bank_directory_sync", %tag, "downloading Razorpay IFSC release");

    let banks_bytes = http_get_bytes(&client, &asset_url(&tag, "banks.json")).await?;
    let sublet_bytes = http_get_bytes(&client, &asset_url(&tag, "sublet.json")).await?;
    let csv_bytes = http_get_bytes(&client, &asset_url(&tag, "IFSC.csv")).await?;

    let banks = parse_banks_json(&banks_bytes)?;
    let sublets = parse_sublet_json(&sublet_bytes)?;
    let rows = parse_ifsc_csv(&csv_bytes, &banks, &sublets)?;
    let count = rows.len();
    upsert_banks(db, &rows).await?;
    mark_success(db, Utc::now()).await?;
    Ok(count)
}

/// How long to sleep before the next sync attempt.
pub fn sleep_until_next_sync(last_success: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Duration {
    match last_success {
        Some(at) => {
            let due = at + chrono::Duration::days(1);
            if due <= now {
                Duration::from_secs(0)
            } else {
                (due - now).to_std().unwrap_or(Duration::from_secs(0))
            }
        }
        None => Duration::from_secs(0),
    }
}

pub const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(60 * 60);

#[cfg(test)]
mod tests {
    use super::*;

    fn csv_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn join_uses_prefix_and_bank_meta() {
        let banks = HashMap::from([(
            "HDFC".to_string(),
            BankMeta {
                ach_credit: true,
                ach_debit: true,
                apbs: true,
                nach_debit: false,
                bank_type: Some("Private".into()),
            },
        )]);
        let sublets = HashMap::new();
        let row = join_branch_row(
            &csv_map(&[
                ("BANK", "HDFC Bank"),
                ("IFSC", "HDFC0001234"),
                ("BRANCH", "Andheri"),
                ("CENTRE", "MUMBAI"),
                ("DISTRICT", "MUMBAI"),
                ("STATE", "MAHARASHTRA"),
                ("ADDRESS", "Andheri West"),
                ("CONTACT", ""),
                ("IMPS", "true"),
                ("RTGS", "true"),
                ("CITY", "MUMBAI"),
                ("ISO3166", "IN-MH"),
                ("NEFT", "true"),
                ("MICR", "400240001"),
                ("UPI", "true"),
                ("SWIFT", ""),
            ]),
            &banks,
            &sublets,
        )
        .expect("row");
        assert_eq!(row.bank_code, "HDFC");
        assert_eq!(row.micr.as_deref(), Some("400240001"));
        assert!(row.ach_credit);
        assert!(!row.nach_debit);
        assert_eq!(row.bank_type.as_deref(), Some("Private"));
        assert!(row.swift.is_none());
    }

    #[test]
    fn join_honours_sublet_and_blank_bank_name() {
        let banks = HashMap::from([(
            "AACX".to_string(),
            BankMeta {
                ach_credit: true,
                ach_debit: false,
                apbs: false,
                nach_debit: false,
                bank_type: Some("O-UCB".into()),
            },
        )]);
        let sublets = HashMap::from([("HDFC0CAACOB".to_string(), "AACX".to_string())]);
        let row = join_branch_row(
            &csv_map(&[
                ("BANK", ""),
                ("IFSC", "HDFC0CAACOB"),
                ("BRANCH", "Main"),
                ("CENTRE", "SURAT"),
                ("DISTRICT", "SURAT"),
                ("STATE", "GUJARAT"),
                ("ADDRESS", "Surat"),
                ("CONTACT", ""),
                ("IMPS", "true"),
                ("RTGS", "false"),
                ("CITY", "SURAT"),
                ("ISO3166", "IN-GJ"),
                ("NEFT", "true"),
                ("MICR", ""),
                ("UPI", "false"),
                ("SWIFT", "ABCDEFGH"),
            ]),
            &banks,
            &sublets,
        )
        .expect("row");
        assert_eq!(row.bank_code, "AACX");
        assert!(row.bank_name.is_none());
        assert!(row.micr.is_none());
        assert_eq!(row.swift.as_deref(), Some("ABCDEFGH"));
        assert!(row.ach_credit);
        assert!(!row.ach_debit);
    }

    #[test]
    fn join_missing_banks_json_defaults_false() {
        let banks = HashMap::new();
        let sublets = HashMap::new();
        let row = join_branch_row(
            &csv_map(&[
                ("BANK", "Unknown"),
                ("IFSC", "ZZZZ0000001"),
                ("BRANCH", "HO"),
                ("CENTRE", "X"),
                ("DISTRICT", "X"),
                ("STATE", "X"),
                ("ADDRESS", "X"),
                ("CONTACT", ""),
                ("IMPS", "false"),
                ("RTGS", "false"),
                ("CITY", "X"),
                ("ISO3166", ""),
                ("NEFT", "false"),
                ("MICR", ""),
                ("UPI", "false"),
                ("SWIFT", ""),
            ]),
            &banks,
            &sublets,
        )
        .expect("row");
        assert_eq!(row.bank_code, "ZZZZ");
        assert!(!row.ach_credit);
        assert!(!row.ach_debit);
        assert!(!row.apbs);
        assert!(!row.nach_debit);
        assert!(row.bank_type.is_none());
    }

    #[test]
    fn sleep_until_respects_24h_window() {
        let now = Utc::now();
        assert_eq!(sleep_until_next_sync(None, now), Duration::from_secs(0));
        let recent = now - chrono::Duration::hours(1);
        let wait = sleep_until_next_sync(Some(recent), now);
        assert!(wait > Duration::from_secs(60 * 60 * 22));
        let old = now - chrono::Duration::hours(25);
        assert_eq!(
            sleep_until_next_sync(Some(old), now),
            Duration::from_secs(0)
        );
    }
}
