//! Journal entry and source document operations.

use std::collections::{HashSet, VecDeque};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Statement, TransactionTrait,
};

use crate::plugins::finance_common::decimal;

use crate::plugins::finance_accounts::{
    account_validation::validate_leaf_account_balance_type,
    balance_type::BalanceType,
    entities::{
        journal::Entity as JournalEntity,
        journal_entry::{self, Entity as JournalEntryEntity},
        journal_entry_item::{self, Entity as JournalEntryItemEntity},
        source_doc::{self},
    },
    source_doc_label::resolve_source_doc_display,
    source_doc_registry::SourceDocRegistry,
};

#[derive(Clone, Debug)]
pub struct JournalLineSpec {
    pub account_id: i64,
    pub amount: Decimal,
}

pub async fn create_source_doc<C: ConnectionTrait>(db: &C, source_doc_type: &str) -> Result<i64> {
    let now = Utc::now();
    let am = source_doc::ActiveModel {
        source_doc_type: Set(source_doc_type.to_string()),
        source_doc_id: Set(0),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    let m = am.insert(db).await?;
    Ok(m.id)
}

pub async fn update_source_doc_id<C: ConnectionTrait>(
    db: &C,
    doc_id: i64,
    source_doc_id: i64,
) -> Result<()> {
    let mut am: source_doc::ActiveModel = source_doc::Entity::find_by_id(doc_id)
        .one(db)
        .await?
        .context("source doc not found")?
        .into();
    am.source_doc_id = Set(source_doc_id);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await?;
    Ok(())
}

pub async fn insert_journal_entry<C: ConnectionTrait>(
    db: &C,
    datetime: DateTime<Utc>,
    journal_id: i64,
    source_doc_id: i64,
    lines: &[JournalLineSpec],
) -> Result<(i64, Vec<journal_entry_item::Model>)> {
    if lines.is_empty() {
        bail!("journal entry requires at least one line");
    }
    let now = Utc::now();
    let je_am = journal_entry::ActiveModel {
        datetime: Set(datetime),
        source_doc_id: Set(source_doc_id),
        journal_id: Set(journal_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    let je = je_am.insert(db).await?;
    let mut items = Vec::with_capacity(lines.len());
    for line in lines {
        let item_am = journal_entry_item::ActiveModel {
            datetime: Set(datetime),
            account_id: Set(line.account_id),
            amount: Set(decimal::normalize(line.amount)),
            journal_entry_id: Set(je.id),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        items.push(item_am.insert(db).await?);
    }
    let balance: Decimal = items.iter().map(|i| i.amount).sum();
    if !decimal::dec_is_zero(balance) {
        bail!("internal error: journal entry does not balance");
    }
    Ok((je.id, items))
}

pub async fn create_journal_entry_with_lines(
    db: &DatabaseConnection,
    datetime: DateTime<Utc>,
    journal_id: i64,
    source_doc_type: &str,
    lines: &[JournalLineSpec],
) -> Result<(i64, Vec<journal_entry_item::Model>)> {
    let txn = db.begin().await?;
    let doc_id = create_source_doc(&txn, source_doc_type).await?;
    let (je_id, items) = insert_journal_entry(&txn, datetime, journal_id, doc_id, lines).await?;
    txn.commit().await?;
    Ok((je_id, items))
}

pub async fn load_journal_entry_lines(
    db: &DatabaseConnection,
    journal_entry_id: i64,
) -> Result<Vec<journal_entry_item::Model>> {
    Ok(JournalEntryItemEntity::find()
        .filter(journal_entry_item::Column::JournalEntryId.eq(journal_entry_id))
        .order_by_desc(journal_entry_item::Column::Id)
        .all(db)
        .await?)
}

pub async fn create_reversing_journal_entry_in_txn<C: ConnectionTrait>(
    db: &C,
    original_entry_id: i64,
    datetime: DateTime<Utc>,
    source_doc_type: &str,
) -> Result<(i64, i64)> {
    let orig = JournalEntryEntity::find_by_id(original_entry_id)
        .one(db)
        .await?
        .context("load journal entry")?;
    let items = JournalEntryItemEntity::find()
        .filter(journal_entry_item::Column::JournalEntryId.eq(original_entry_id))
        .order_by_asc(journal_entry_item::Column::Id)
        .all(db)
        .await?;
    if items.is_empty() {
        bail!("journal entry has no lines to reverse");
    }
    let doc_id = create_source_doc(db, source_doc_type).await?;
    let rev_lines: Vec<JournalLineSpec> = items
        .iter()
        .map(|it| JournalLineSpec {
            account_id: it.account_id,
            amount: decimal::dec_neg(it.amount),
        })
        .collect();
    let (rev_id, _) =
        insert_journal_entry(db, datetime, orig.journal_id, doc_id, &rev_lines).await?;
    Ok((doc_id, rev_id))
}

pub async fn validate_leaf_account_for_posting(
    db: &DatabaseConnection,
    account_id: i64,
    want: BalanceType,
    label: &str,
) -> Result<()> {
    validate_leaf_account_balance_type(db, account_id, want, label)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

pub fn debit_balance_type() -> BalanceType {
    BalanceType::Debit
}

pub fn credit_balance_type() -> BalanceType {
    BalanceType::Credit
}

#[derive(Clone, Debug, Default)]
pub struct CascadeGraph {
    pub journal_entry_ids: HashSet<i64>,
    pub source_doc_ids: HashSet<i64>,
    pub posted_invoice_ids: HashSet<i64>,
    pub payment_ids: HashSet<i64>,
    pub payment_batch_ids: HashSet<i64>,
    pub credit_note_ids: HashSet<i64>,
    pub cancelled_invoice_ids: HashSet<i64>,
}

#[derive(Clone, Debug)]
pub struct CascadeDeleteItem {
    pub kind: String,
    pub label: String,
    pub url: String,
}

#[derive(Default)]
struct CascadeWork {
    journal_entries: VecDeque<i64>,
    posted_invoices: VecDeque<i64>,
    payment_batches: VecDeque<i64>,
    credit_notes: VecDeque<i64>,
    cancelled_invoices: VecDeque<i64>,
}

/// Walk journal entries, source docs, and related finance rows without deleting.
pub async fn collect_journal_entry_cascade<C: ConnectionTrait>(
    db: &C,
    entry_id: i64,
) -> Result<CascadeGraph> {
    let mut graph = CascadeGraph::default();
    let mut work = CascadeWork::default();
    work.journal_entries.push_back(entry_id);

    loop {
        if let Some(je_id) = work.journal_entries.pop_front() {
            if graph.journal_entry_ids.insert(je_id) {
                seed_from_journal_entry(db, je_id, &mut graph, &mut work).await?;
            }
            continue;
        }
        if let Some(posted_id) = work.posted_invoices.pop_front() {
            if graph.posted_invoice_ids.insert(posted_id) {
                seed_from_posted_invoice(db, posted_id, &mut graph, &mut work).await?;
            }
            continue;
        }
        if let Some(batch_id) = work.payment_batches.pop_front() {
            if graph.payment_batch_ids.insert(batch_id) {
                seed_from_payment_batch(db, batch_id, &mut graph, &mut work).await?;
            }
            continue;
        }
        if let Some(cn_id) = work.credit_notes.pop_front() {
            if graph.credit_note_ids.insert(cn_id) {
                seed_from_credit_note(db, cn_id, &mut work).await?;
            }
            continue;
        }
        if let Some(cancelled_id) = work.cancelled_invoices.pop_front() {
            if graph.cancelled_invoice_ids.insert(cancelled_id) {
                seed_from_cancelled_invoice(db, cancelled_id, &mut work).await?;
            }
            continue;
        }
        break;
    }

    Ok(graph)
}

/// Hard-delete a journal entry and every finance row reachable through FKs that
/// reference journal entries / items, recursively collecting sibling journal entries.
///
/// Credit notes do not take the original posting with them: only the reversing
/// entry is collected. Cancelled invoices that require the credit note are still
/// removed.
pub async fn delete_journal_entry_recursive(db: &DatabaseConnection, entry_id: i64) -> Result<()> {
    let txn = db.begin().await?;
    let graph = collect_journal_entry_cascade(&txn, entry_id).await?;
    apply_cascade_deletes(&txn, &graph).await?;
    txn.commit().await?;
    Ok(())
}

/// Labeled rows for the delete confirmation modal.
pub async fn cascade_delete_preview(
    db: &DatabaseConnection,
    registry: &SourceDocRegistry,
    graph: &CascadeGraph,
) -> Result<Vec<CascadeDeleteItem>> {
    let mut items = Vec::new();

    for id in sorted_ids(&graph.journal_entry_ids) {
        let Some(entry) = JournalEntryEntity::find_by_id(id).one(db).await? else {
            continue;
        };
        let journal_name = JournalEntity::find_by_id(entry.journal_id)
            .one(db)
            .await?
            .map(|j| j.name)
            .unwrap_or_else(|| "—".into());
        let dt = entry.datetime.format("%Y-%m-%d %H:%M").to_string();
        items.push(CascadeDeleteItem {
            kind: "Journal entry".into(),
            label: format!("Entry #{id} · {journal_name} · {dt}"),
            url: format!("/finance/journal-entries/{id}"),
        });
    }

    for id in sorted_ids(&graph.source_doc_ids) {
        let display = resolve_source_doc_display(db, registry, id).await;
        items.push(CascadeDeleteItem {
            kind: "Source document".into(),
            label: display.summary_label(),
            url: display.detail_url,
        });
    }

    for id in sorted_ids(&graph.posted_invoice_ids) {
        let number =
            query_optional_text(db, "SELECT number FROM posted_invoices WHERE id = $1", id)
                .await?
                .unwrap_or_default();
        items.push(CascadeDeleteItem {
            kind: "Posted invoice".into(),
            label: invoice_label(id, &number),
            url: registry
                .type_detail_url("p_finance_invoices.PostedInvoice", id)
                .unwrap_or_else(|| format!("/finance-invoices/posted/{id}")),
        });
    }

    for id in sorted_ids(&graph.payment_ids) {
        items.push(CascadeDeleteItem {
            kind: "Payment".into(),
            label: format!("Payment #{id}"),
            url: registry
                .type_detail_url("p_finance_invoices.Payment", id)
                .unwrap_or_else(|| format!("/finance-invoices/payments/{id}")),
        });
    }

    for id in sorted_ids(&graph.payment_batch_ids) {
        items.push(CascadeDeleteItem {
            kind: "Payment batch".into(),
            label: format!("Batch #{id}"),
            url: registry
                .type_detail_url("p_finance_invoices.PaymentBatch", id)
                .unwrap_or_else(|| format!("/finance-invoices/payment-batches/{id}")),
        });
    }

    for id in sorted_ids(&graph.credit_note_ids) {
        items.push(CascadeDeleteItem {
            kind: "Credit note".into(),
            label: format!("Credit note #{id}"),
            url: registry
                .type_detail_url("p_finance_creditnotes.CreditNote", id)
                .unwrap_or_else(|| format!("/finance-credit-notes/c/{id}")),
        });
    }

    for id in sorted_ids(&graph.cancelled_invoice_ids) {
        let number = query_optional_text(
            db,
            "SELECT number FROM cancelled_invoices WHERE id = $1",
            id,
        )
        .await?
        .unwrap_or_default();
        items.push(CascadeDeleteItem {
            kind: "Cancelled invoice".into(),
            label: invoice_label(id, &number),
            url: format!("/finance-invoices/cancelled/{id}"),
        });
    }

    Ok(items)
}

fn invoice_label(id: i64, number: &str) -> String {
    if number.is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

fn sorted_ids(ids: &HashSet<i64>) -> Vec<i64> {
    let mut out: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    out.sort_unstable();
    out
}

async fn seed_from_journal_entry<C: ConnectionTrait>(
    db: &C,
    je_id: i64,
    graph: &mut CascadeGraph,
    work: &mut CascadeWork,
) -> Result<()> {
    push_ids(
        &mut work.credit_notes,
        query_i64_col(
            db,
            "SELECT id FROM credit_notes \
             WHERE journal_entry_id = $1 OR reversed_journal_entry_id = $1",
            je_id,
        )
        .await?,
    );

    push_ids(
        &mut work.posted_invoices,
        query_i64_col(
            db,
            "SELECT id FROM posted_invoices WHERE journal_entry_id = $1",
            je_id,
        )
        .await?,
    );

    let payment_rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, posted_invoice_id, payment_batch_id FROM payments \
             WHERE journal_entry_id = $1",
            [je_id.into()],
        ))
        .await?;
    for row in payment_rows {
        let payment_id: i64 = row.try_get("", "id")?;
        let posted_invoice_id: i64 = row.try_get("", "posted_invoice_id")?;
        let payment_batch_id: Option<i64> = row.try_get("", "payment_batch_id")?;
        graph.payment_ids.insert(payment_id);
        work.posted_invoices.push_back(posted_invoice_id);
        if let Some(batch_id) = payment_batch_id {
            work.payment_batches.push_back(batch_id);
        }
    }

    push_ids(
        &mut work.payment_batches,
        query_i64_col(
            db,
            "SELECT id FROM payment_batches WHERE journal_entry_id = $1",
            je_id,
        )
        .await?,
    );

    let item_ids = query_i64_col(
        db,
        "SELECT id FROM journal_entry_items WHERE journal_entry_id = $1",
        je_id,
    )
    .await?;
    for item_id in item_ids {
        push_ids(
            &mut work.posted_invoices,
            query_i64_col(
                db,
                "SELECT DISTINCT posted_invoice_id FROM posted_invoice_lines \
                 WHERE journal_entry_item_id = $1",
                item_id,
            )
            .await?,
        );
        push_ids(
            &mut work.cancelled_invoices,
            query_i64_col(
                db,
                "SELECT DISTINCT cancelled_invoice_id FROM cancelled_invoice_lines \
                 WHERE journal_entry_item_id = $1",
                item_id,
            )
            .await?,
        );
    }

    if let Some(doc_id) = query_optional_i64(
        db,
        "SELECT source_doc_id FROM journal_entries WHERE id = $1",
        je_id,
    )
    .await?
    {
        if doc_id > 0 {
            graph.source_doc_ids.insert(doc_id);
            push_ids(
                &mut work.journal_entries,
                query_i64_col(
                    db,
                    "SELECT id FROM journal_entries WHERE source_doc_id = $1",
                    doc_id,
                )
                .await?,
            );
        }
    }

    Ok(())
}

async fn seed_from_posted_invoice<C: ConnectionTrait>(
    db: &C,
    posted_id: i64,
    graph: &mut CascadeGraph,
    work: &mut CascadeWork,
) -> Result<()> {
    if let Some(je_id) = query_optional_i64(
        db,
        "SELECT journal_entry_id FROM posted_invoices WHERE id = $1",
        posted_id,
    )
    .await?
    {
        push_je(&mut work.journal_entries, je_id);
    }

    let payment_rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, journal_entry_id, payment_batch_id FROM payments \
             WHERE posted_invoice_id = $1",
            [posted_id.into()],
        ))
        .await?;
    for row in payment_rows {
        let payment_id: i64 = row.try_get("", "id")?;
        let je_id: i64 = row.try_get("", "journal_entry_id")?;
        let payment_batch_id: Option<i64> = row.try_get("", "payment_batch_id")?;
        graph.payment_ids.insert(payment_id);
        push_je(&mut work.journal_entries, je_id);
        if let Some(batch_id) = payment_batch_id {
            work.payment_batches.push_back(batch_id);
        }
    }

    push_ids(
        &mut work.cancelled_invoices,
        query_i64_col(
            db,
            "SELECT id FROM cancelled_invoices WHERE posted_invoice_id = $1",
            posted_id,
        )
        .await?,
    );

    Ok(())
}

async fn seed_from_payment_batch<C: ConnectionTrait>(
    db: &C,
    batch_id: i64,
    graph: &mut CascadeGraph,
    work: &mut CascadeWork,
) -> Result<()> {
    if let Some(je_id) = query_optional_i64(
        db,
        "SELECT journal_entry_id FROM payment_batches WHERE id = $1",
        batch_id,
    )
    .await?
    {
        push_je(&mut work.journal_entries, je_id);
    }

    let payment_rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, posted_invoice_id, journal_entry_id FROM payments \
             WHERE payment_batch_id = $1",
            [batch_id.into()],
        ))
        .await?;
    for row in payment_rows {
        let payment_id: i64 = row.try_get("", "id")?;
        let posted_invoice_id: i64 = row.try_get("", "posted_invoice_id")?;
        let je_id: i64 = row.try_get("", "journal_entry_id")?;
        graph.payment_ids.insert(payment_id);
        push_je(&mut work.journal_entries, je_id);
        work.posted_invoices.push_back(posted_invoice_id);
    }

    Ok(())
}

async fn seed_from_cancelled_invoice<C: ConnectionTrait>(
    db: &C,
    cancelled_id: i64,
    work: &mut CascadeWork,
) -> Result<()> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT credit_note_id FROM cancelled_invoices WHERE id = $1",
            [cancelled_id.into()],
        ))
        .await?;
    let Some(row) = row else {
        return Ok(());
    };
    // Do not walk posted_invoice_id: that posting and its original journal
    // entry must survive deleting a credit note / reversing entry.
    let credit_note_id: i64 = row.try_get("", "credit_note_id")?;
    work.credit_notes.push_back(credit_note_id);
    Ok(())
}

async fn seed_from_credit_note<C: ConnectionTrait>(
    db: &C,
    credit_note_id: i64,
    work: &mut CascadeWork,
) -> Result<()> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT reversed_journal_entry_id FROM credit_notes WHERE id = $1",
            [credit_note_id.into()],
        ))
        .await?;
    let Some(row) = row else {
        return Ok(());
    };
    // Only the reversal belongs to the credit note. journal_entry_id is the
    // original posting being reversed and must not be collected.
    let reversed_journal_entry_id: i64 = row.try_get("", "reversed_journal_entry_id")?;
    push_je(&mut work.journal_entries, reversed_journal_entry_id);

    push_ids(
        &mut work.cancelled_invoices,
        query_i64_col(
            db,
            "SELECT id FROM cancelled_invoices WHERE credit_note_id = $1",
            credit_note_id,
        )
        .await?,
    );

    Ok(())
}

fn push_je(queue: &mut VecDeque<i64>, je_id: i64) {
    if je_id > 0 {
        queue.push_back(je_id);
    }
}

fn push_ids(queue: &mut VecDeque<i64>, ids: Vec<i64>) {
    for id in ids {
        if id > 0 {
            queue.push_back(id);
        }
    }
}

async fn apply_cascade_deletes<C: ConnectionTrait>(db: &C, graph: &CascadeGraph) -> Result<()> {
    // Settlements keyed by posted invoice
    for &posted_id in &graph.posted_invoice_ids {
        delete_where(
            db,
            "DELETE FROM paid_invoices WHERE posted_invoice_id = $1",
            posted_id,
        )
        .await?;
        delete_where(
            db,
            "DELETE FROM partially_paid_invoices WHERE posted_invoice_id = $1",
            posted_id,
        )
        .await?;
    }

    delete_by_ids(db, "payments", &graph.payment_ids).await?;
    delete_by_ids(db, "payment_batches", &graph.payment_batch_ids).await?;

    for &cancelled_id in &graph.cancelled_invoice_ids {
        delete_where(
            db,
            "DELETE FROM cancelled_invoice_lines WHERE cancelled_invoice_id = $1",
            cancelled_id,
        )
        .await?;
    }
    delete_by_ids(db, "cancelled_invoices", &graph.cancelled_invoice_ids).await?;

    for &posted_id in &graph.posted_invoice_ids {
        delete_where(
            db,
            "DELETE FROM posted_invoice_lines WHERE posted_invoice_id = $1",
            posted_id,
        )
        .await?;
    }
    delete_by_ids(db, "posted_invoices", &graph.posted_invoice_ids).await?;
    delete_by_ids(db, "credit_notes", &graph.credit_note_ids).await?;

    for &je_id in &graph.journal_entry_ids {
        delete_where(
            db,
            "DELETE FROM journal_entry_items WHERE journal_entry_id = $1",
            je_id,
        )
        .await?;
    }
    delete_by_ids(db, "journal_entries", &graph.journal_entry_ids).await?;
    delete_by_ids(db, "source_docs", &graph.source_doc_ids).await?;

    Ok(())
}

async fn delete_by_ids<C: ConnectionTrait>(db: &C, table: &str, ids: &HashSet<i64>) -> Result<()> {
    for &id in ids {
        delete_where(db, &format!("DELETE FROM {table} WHERE id = $1"), id).await?;
    }
    Ok(())
}

async fn delete_where<C: ConnectionTrait>(db: &C, sql: &str, id: i64) -> Result<()> {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        [id.into()],
    ))
    .await?;
    Ok(())
}

async fn query_i64_col<C: ConnectionTrait>(db: &C, sql: &str, id: i64) -> Result<Vec<i64>> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [id.into()],
        ))
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        // Column alias varies; take the first column value.
        let cols = row.column_names();
        let name = cols.first().map(|s| s.as_str()).unwrap_or("id");
        out.push(row.try_get("", name)?);
    }
    Ok(out)
}

async fn query_optional_text<C: ConnectionTrait>(
    db: &C,
    sql: &str,
    id: i64,
) -> Result<Option<String>> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [id.into()],
        ))
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let cols = row.column_names();
    let name = cols.first().map(|s| s.as_str()).unwrap_or("number");
    Ok(Some(row.try_get("", name)?))
}

async fn query_optional_i64<C: ConnectionTrait>(db: &C, sql: &str, id: i64) -> Result<Option<i64>> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [id.into()],
        ))
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let cols = row.column_names();
    let name = cols.first().map(|s| s.as_str()).unwrap_or("id");
    Ok(Some(row.try_get("", name)?))
}
