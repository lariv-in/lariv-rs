//! Create, update, and delete a custom source document and its journal entry.

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::Deserialize;
use serde_json::Value;

use lariv_plugin_finance_common::decimal::{self, decimal_display};

use crate::{
    entities::{
        account::{self, Entity as AccountEntity},
        custom_source_doc::{self, CUSTOM_SOURCE_DOC_TYPE, Entity as CustomSourceDocEntity},
        journal::{self, Entity as JournalEntity},
        journal_entry::{self, Entity as JournalEntryEntity},
        journal_entry_item::{self, Entity as JournalEntryItemEntity},
    },
    logic::journal::{
        JournalLineSpec, create_source_doc, delete_journal_entry_recursive, insert_journal_entry,
        update_source_doc_id,
    },
};

const IMMUTABLE_JOURNAL: &str = "This journal is immutable. Enable Mutable on the journal edit page before changing source documents.";

/// Blank row so the line editor opens with one empty line.
pub fn empty_lines_json() -> String {
    r#"[{"account_id":0,"debit":"","credit":"","account_label":"","fk_slot":"journal-line-0"}]"#
        .to_string()
}

/// Journal lines submitted from the line editor, with optional document-line anchors.
///
/// `anchors` is parallel to `lines`. `Some(i)` means that row is the anchor for
/// document line `i` (goods lines first, then service lines).
pub struct SubmittedJournalLines {
    pub lines: Vec<JournalLineSpec>,
    pub anchors: Vec<Option<usize>>,
}

/// Parse the line editor JSON into balanced signed journal lines.
///
/// A blank row (no account and no amounts) is skipped. Debit is a positive
/// amount and credit is a negative amount. The kept lines must net to zero.
pub fn parse_journal_lines(raw: &str) -> Result<Vec<JournalLineSpec>, String> {
    Ok(parse_submitted_journal_lines(raw)?.lines)
}

/// Parse editor JSON and keep each row's `doc_line` anchor.
pub fn parse_submitted_journal_lines(raw: &str) -> Result<SubmittedJournalLines, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("At least one journal line is required".into());
    }
    let rows: Vec<RawLine> =
        serde_json::from_str(raw).map_err(|_err| "Journal lines could not be read".to_string())?;
    let mut lines = Vec::new();
    let mut anchors = Vec::new();
    let mut balance = Decimal::ZERO;
    for (idx, row) in rows.iter().enumerate() {
        let n = idx + 1;
        let debit_raw = row.debit.trim();
        let credit_raw = row.credit.trim();
        let blank_amounts = debit_raw.is_empty() && credit_raw.is_empty();
        if row.account_id <= 0 && blank_amounts {
            continue;
        }
        if row.account_id <= 0 {
            return Err(format!("Line {n}: account is required"));
        }
        let debit = parse_amount(&format!("Line {n} debit"), debit_raw)?;
        let credit = parse_amount(&format!("Line {n} credit"), credit_raw)?;
        if !debit.is_zero() && !credit.is_zero() {
            return Err(format!("Line {n}: enter a debit or a credit, not both"));
        }
        if debit.is_zero() && credit.is_zero() {
            return Err(format!("Line {n}: enter a debit or a credit"));
        }
        let amount = if !debit.is_zero() {
            debit
        } else {
            decimal::dec_neg(credit)
        };
        balance = decimal::dec_sum(balance, amount);
        lines.push(JournalLineSpec {
            account_id: row.account_id,
            amount,
        });
        anchors.push(row.doc_line.and_then(|n| usize::try_from(n).ok()));
    }
    if lines.is_empty() {
        return Err("At least one journal line is required".into());
    }
    if !decimal::dec_is_zero(balance) {
        return Err("Debits must equal credits".into());
    }
    Ok(SubmittedJournalLines { lines, anchors })
}

/// Map each document line to a submitted journal row.
///
/// A document line whose anchor was removed points at the first submitted row.
pub fn doc_line_indexes(doc_count: usize, anchors: &[Option<usize>]) -> Vec<usize> {
    let last = anchors.len().saturating_sub(1);
    (0..doc_count)
        .map(|doc| {
            anchors
                .iter()
                .position(|anchor| *anchor == Some(doc))
                .unwrap_or(0)
                .min(last)
        })
        .collect()
}

/// Position of `item_id` in an id-ordered journal entry.
pub fn index_of_item(item_ids: &[i64], item_id: i64) -> Result<usize, String> {
    item_ids
        .iter()
        .position(|id| *id == item_id)
        .ok_or_else(|| format!("journal line {item_id} is not on this journal entry"))
}

/// Stored journal items in insertion order, as signed lines.
pub async fn stored_journal_line_specs(
    db: &DatabaseConnection,
    journal_entry_id: i64,
) -> Result<Vec<(i64, JournalLineSpec)>, String> {
    let items = JournalEntryItemEntity::find()
        .filter(journal_entry_item::Column::JournalEntryId.eq(journal_entry_id))
        .order_by_asc(journal_entry_item::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if items.is_empty() {
        return Err(format!("journal entry {journal_entry_id} has no lines"));
    }
    Ok(items
        .into_iter()
        .map(|item| {
            (
                item.id,
                JournalLineSpec {
                    account_id: item.account_id,
                    amount: item.amount,
                },
            )
        })
        .collect())
}

/// Editor JSON for evaluated journal lines.
///
/// `anchor_indexes[doc_line]` is the spec index that anchors that document line.
/// `slot_prefix` keeps account-picker ids unique when several editors share a page.
pub async fn journal_specs_editor_json(
    db: &DatabaseConnection,
    lines: &[JournalLineSpec],
    anchor_indexes: &[usize],
    slot_prefix: &str,
) -> Result<String, String> {
    if lines.is_empty() {
        return Ok(empty_lines_json());
    }
    let mut rows = Vec::with_capacity(lines.len());
    for (idx, line) in lines.iter().enumerate() {
        let acct = AccountEntity::find_by_id(line.account_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?;
        let account_label = acct
            .as_ref()
            .map(account_label)
            .unwrap_or_else(|| format!("Account #{}", line.account_id));
        let (debit, credit) = if line.amount.is_sign_negative() {
            (String::new(), decimal_display(line.amount.abs()))
        } else if line.amount.is_zero() {
            (String::new(), String::new())
        } else {
            (decimal_display(line.amount), String::new())
        };
        let doc_line = anchor_indexes
            .iter()
            .position(|anchor| *anchor == idx)
            .and_then(|n| i64::try_from(n).ok());
        rows.push(LineJson {
            account_id: line.account_id,
            debit,
            credit,
            account_label,
            fk_slot: format!("{slot_prefix}-{idx}"),
            doc_line,
        });
    }
    serde_json::to_string(&rows).map_err(|e| e.to_string())
}

/// Each line account must exist and must not be a group account.
pub async fn ensure_posting_accounts(
    db: &DatabaseConnection,
    lines: &[JournalLineSpec],
) -> Result<(), String> {
    for (idx, line) in lines.iter().enumerate() {
        let n = idx + 1;
        let acct = AccountEntity::find_by_id(line.account_id)
            .one(db)
            .await
            .map_err(|e| format!("Line {n}: {e}"))?
            .ok_or_else(|| format!("Line {n}: account not found"))?;
        if acct.is_group {
            return Err(format!(
                "Line {n}: group accounts cannot be used for posting ({})",
                account_label(&acct)
            ));
        }
    }
    Ok(())
}

pub async fn lines_form_value(
    db: &DatabaseConnection,
    journal_entry_id: i64,
) -> Result<String, String> {
    let items = JournalEntryItemEntity::find()
        .filter(journal_entry_item::Column::JournalEntryId.eq(journal_entry_id))
        .order_by_asc(journal_entry_item::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if items.is_empty() {
        return Ok(empty_lines_json());
    }
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        let acct = AccountEntity::find_by_id(item.account_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?;
        let account_label = acct
            .as_ref()
            .map(account_label)
            .unwrap_or_else(|| format!("Account #{}", item.account_id));
        let (debit, credit) = if item.amount.is_sign_negative() {
            (String::new(), decimal_display(item.amount.abs()))
        } else if item.amount.is_zero() {
            (String::new(), String::new())
        } else {
            (decimal_display(item.amount), String::new())
        };
        rows.push(LineJson {
            account_id: item.account_id,
            debit,
            credit,
            account_label,
            fk_slot: format!("journal-line-{}", item.id),
            doc_line: None,
        });
    }
    serde_json::to_string(&rows).map_err(|e| e.to_string())
}

pub async fn create_custom_source_doc(
    db: &DatabaseConnection,
    journal_id: i64,
    title: &str,
    content: &str,
    created_by_id: i64,
    lines: &[JournalLineSpec],
) -> Result<custom_source_doc::Model, String> {
    let journal = load_journal(db, journal_id).await?;
    if !journal.is_mutable {
        return Err(IMMUTABLE_JOURNAL.into());
    }
    if created_by_id <= 0 {
        return Err("Signed-in user is required".into());
    }
    let title = title.trim();
    if title.is_empty() {
        return Err("Title is required".into());
    }
    let now = Utc::now();
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let doc_id = create_source_doc(&txn, CUSTOM_SOURCE_DOC_TYPE)
        .await
        .map_err(|e| e.to_string())?;
    let (entry_id, _) = insert_journal_entry(&txn, now, journal_id, doc_id, lines)
        .await
        .map_err(|e| e.to_string())?;
    let model = custom_source_doc::ActiveModel {
        title: Set(title.to_string()),
        content: Set(content.to_string()),
        created_by_id: Set(created_by_id),
        journal_entry_id: Set(entry_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;
    update_source_doc_id(&txn, doc_id, model.id)
        .await
        .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(model)
}

pub async fn update_custom_source_doc(
    db: &DatabaseConnection,
    id: i64,
    title: &str,
    content: &str,
    lines: &[JournalLineSpec],
) -> Result<(), String> {
    let model = CustomSourceDocEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Transaction not found".to_string())?;
    let entry = JournalEntryEntity::find_by_id(model.journal_entry_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Journal entry not found".to_string())?;
    let journal = load_journal(db, entry.journal_id).await?;
    if !journal.is_mutable {
        return Err(IMMUTABLE_JOURNAL.into());
    }
    let title = title.trim();
    if title.is_empty() {
        return Err("Title is required".into());
    }
    let now = Utc::now();
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    replace_lines(&txn, &entry, lines).await?;
    let mut am: custom_source_doc::ActiveModel = model.into();
    am.title = Set(title.to_string());
    am.content = Set(content.to_string());
    am.updated_at = Set(Some(now));
    am.update(&txn).await.map_err(|e| e.to_string())?;
    let mut entry_am: journal_entry::ActiveModel = entry.into();
    entry_am.updated_at = Set(Some(now));
    entry_am.update(&txn).await.map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_custom_source_doc(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    let model = CustomSourceDocEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Transaction not found".to_string())?;
    let entry = JournalEntryEntity::find_by_id(model.journal_entry_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Journal entry not found".to_string())?;
    let journal = load_journal(db, entry.journal_id).await?;
    if !journal.is_mutable {
        return Err(IMMUTABLE_JOURNAL.into());
    }
    delete_journal_entry_recursive(db, entry.id)
        .await
        .map_err(|e| e.to_string())
}

async fn load_journal(db: &DatabaseConnection, journal_id: i64) -> Result<journal::Model, String> {
    JournalEntity::find_by_id(journal_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Journal not found".to_string())
}

async fn replace_lines<C: ConnectionTrait>(
    db: &C,
    entry: &journal_entry::Model,
    lines: &[JournalLineSpec],
) -> Result<(), String> {
    if lines.is_empty() {
        return Err("At least one journal line is required".into());
    }
    JournalEntryItemEntity::delete_many()
        .filter(journal_entry_item::Column::JournalEntryId.eq(entry.id))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now();
    for line in lines {
        journal_entry_item::ActiveModel {
            datetime: Set(entry.datetime),
            account_id: Set(line.account_id),
            amount: Set(decimal::normalize(line.amount)),
            journal_entry_id: Set(entry.id),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn parse_amount(label: &str, raw: &str) -> Result<Decimal, String> {
    if raw.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let amount = raw
        .trim()
        .parse::<Decimal>()
        .map_err(|_err| format!("{label} is not a number"))?;
    if amount.is_sign_negative() {
        return Err(format!("{label} cannot be negative"));
    }
    Ok(decimal::normalize(amount))
}

fn account_label(acct: &account::Model) -> String {
    format!("{} — {}", acct.code, acct.name)
}

#[derive(Deserialize)]
struct RawLine {
    #[serde(default, deserialize_with = "flex_i64")]
    account_id: i64,
    #[serde(default, deserialize_with = "flex_string")]
    debit: String,
    #[serde(default, deserialize_with = "flex_string")]
    credit: String,
    #[serde(default)]
    doc_line: Option<i64>,
}

#[derive(serde::Serialize)]
struct LineJson {
    account_id: i64,
    debit: String,
    credit: String,
    account_label: String,
    fk_slot: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc_line: Option<i64>,
}

fn flex_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(0),
        Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom("account id")),
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() {
                Ok(0)
            } else {
                s.parse().map_err(serde::de::Error::custom)
            }
        }
        _ => Err(serde::de::Error::custom("account id")),
    }
}

fn flex_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(String::new()),
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(serde::de::Error::custom("amount")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_debit_and_credit() {
        let lines = parse_journal_lines(
            r#"[{"account_id":1,"debit":"10.5","credit":""},{"account_id":2,"debit":"","credit":"10.50"}]"#,
        )
        .unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].account_id, 1);
        assert_eq!(lines[0].amount, Decimal::new(105, 1));
        assert!(lines[1].amount.is_sign_negative());
        assert!(decimal::dec_is_zero(decimal::dec_sum(
            lines[0].amount,
            lines[1].amount
        )));
    }

    #[test]
    fn skips_blank_row_and_rejects_imbalance() {
        let err = parse_journal_lines(
            r#"[{"account_id":0,"debit":"","credit":""},{"account_id":1,"debit":"5","credit":""}]"#,
        )
        .unwrap_err();
        assert_eq!(err, "Debits must equal credits");
    }

    #[test]
    fn keeps_doc_line_anchors_and_falls_back_when_removed() {
        let parsed = parse_submitted_journal_lines(
            r#"[{"account_id":1,"debit":"5","credit":"","doc_line":0},{"account_id":0,"debit":"","credit":""},{"account_id":2,"debit":"","credit":"5","doc_line":1}]"#,
        )
        .unwrap();
        assert_eq!(parsed.anchors, vec![Some(0), Some(1)]);
        assert_eq!(doc_line_indexes(2, &parsed.anchors), vec![0, 1]);
        assert_eq!(doc_line_indexes(3, &parsed.anchors), vec![0, 1, 0]);
    }

    #[test]
    fn rejects_both_sides_and_empty() {
        assert!(
            parse_journal_lines(r#"[{"account_id":1,"debit":"1","credit":"1"}]"#)
                .unwrap_err()
                .contains("not both")
        );
        assert!(
            parse_journal_lines("[]")
                .unwrap_err()
                .contains("At least one")
        );
    }
}
