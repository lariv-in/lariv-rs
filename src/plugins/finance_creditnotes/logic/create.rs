//! Credit note creation with automatic journal reversal (Go BeforeCreate/AfterCreate).

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, TransactionTrait};

use crate::plugins::finance_accounts::logic::journal::{
    create_reversing_journal_entry_in_txn, update_source_doc_id,
};

use crate::plugins::finance_creditnotes::entities::credit_note::{
    self, CREDIT_NOTE_SOURCE_DOC_TYPE,
};

pub struct CreateCreditNoteInput {
    pub datetime: DateTime<Utc>,
    pub reason: String,
    pub journal_entry_id: i64,
}

pub async fn create_credit_note(
    db: &DatabaseConnection,
    input: CreateCreditNoteInput,
) -> Result<credit_note::Model> {
    if input.journal_entry_id == 0 {
        bail!("journal entry is required");
    }
    let now = Utc::now();
    let dt = if input.datetime.timestamp() == 0 {
        now
    } else {
        input.datetime
    };

    let txn = db.begin().await?;
    let (doc_id, reversed_id) = create_reversing_journal_entry_in_txn(
        &txn,
        input.journal_entry_id,
        dt,
        CREDIT_NOTE_SOURCE_DOC_TYPE,
    )
    .await
    .context("create reversal")?;

    let am = credit_note::ActiveModel {
        datetime: Set(dt),
        reason: Set(if input.reason.is_empty() {
            None
        } else {
            Some(input.reason)
        }),
        journal_entry_id: Set(input.journal_entry_id),
        reversed_journal_entry_id: Set(reversed_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    let cn = am.insert(&txn).await?;
    update_source_doc_id(&txn, doc_id, cn.id).await?;
    txn.commit().await?;
    Ok(cn)
}
