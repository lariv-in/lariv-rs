//! Custom source document type registered by the accounts hub.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, QueryOrder,
};

use crate::{
    entities::{
        custom_source_doc::{self, CUSTOM_SOURCE_DOC_TYPE, Entity as CustomSourceDocEntity},
        journal_entry_item::{self, Entity as JournalEntryItemEntity},
    },
    routes::CustomSourceDocDetailRouteTag,
    source_doc_registry::{
        SourceDocInstance, SourceDocJournalLine, SourceDocJournalSpec, SourceDocRegistry,
        SourceDocType,
    },
};

pub fn register(registry: SourceDocRegistry) -> SourceDocRegistry {
    registry.register(Arc::new(CustomSourceDocType))
}

struct CustomSourceDocType;

struct CustomSourceDocInstance {
    id: i64,
    title: String,
    created_at: DateTime<Utc>,
}

impl SourceDocInstance for CustomSourceDocInstance {
    fn source_doc_type(&self) -> &str {
        CUSTOM_SOURCE_DOC_TYPE
    }

    fn source_doc_id(&self) -> i64 {
        self.id
    }

    fn display_name(&self) -> String {
        if self.title.trim().is_empty() {
            format!("Custom source doc #{}", self.id)
        } else {
            self.title.clone()
        }
    }

    fn detail_url(&self) -> String {
        CustomSourceDocDetailRouteTag::new(self.id).url()
    }

    fn datetime(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[async_trait]
impl SourceDocType for CustomSourceDocType {
    fn source_doc_type(&self) -> &str {
        CUSTOM_SOURCE_DOC_TYPE
    }

    fn display_name(&self) -> &str {
        "Custom Source Doc"
    }

    fn detail_url(&self, id: i64) -> String {
        CustomSourceDocDetailRouteTag::new(id).url()
    }

    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>> {
        let model = CustomSourceDocEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("custom source doc {id} not found"))?;
        Ok(Arc::new(CustomSourceDocInstance {
            id: model.id,
            title: model.title,
            created_at: model.created_at.unwrap_or_else(Utc::now),
        }))
    }

    async fn journal_lines(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<SourceDocJournalSpec> {
        let model = CustomSourceDocEntity::find_by_id(id)
            .one(db)
            .await?
            .with_context(|| format!("custom source doc {id} not found"))?;
        if model.journal_entry_id <= 0 {
            bail!("custom source doc {id} has no journal entry");
        }
        let items = JournalEntryItemEntity::find()
            .filter(journal_entry_item::Column::JournalEntryId.eq(model.journal_entry_id))
            .order_by_asc(journal_entry_item::Column::Id)
            .all(db)
            .await?;
        if items.is_empty() {
            bail!(
                "custom source doc {id} journal entry {} has no lines",
                model.journal_entry_id
            );
        }
        let datetime = items
            .first()
            .map(|item| item.datetime)
            .or(model.created_at)
            .unwrap_or_else(Utc::now);
        Ok(SourceDocJournalSpec {
            datetime,
            lines: items
                .into_iter()
                .map(|item| SourceDocJournalLine {
                    account_id: item.account_id,
                    amount: item.amount,
                })
                .collect(),
            line_item_indexes: Vec::new(),
        })
    }

    async fn adopt_journal_entry(
        &self,
        txn: &DatabaseTransaction,
        backing_id: i64,
        _journal_id: i64,
        journal_entry_id: i64,
        _item_ids: &[i64],
        _line_item_indexes: &[usize],
    ) -> Result<()> {
        let model = CustomSourceDocEntity::find_by_id(backing_id)
            .one(txn)
            .await?
            .with_context(|| format!("custom source doc {backing_id} not found"))?;
        let mut am: custom_source_doc::ActiveModel = model.into();
        am.journal_entry_id = Set(journal_entry_id);
        am.updated_at = Set(Some(Utc::now()));
        am.update(txn).await?;
        Ok(())
    }
}
