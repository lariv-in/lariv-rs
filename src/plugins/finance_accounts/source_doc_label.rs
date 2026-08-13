//! Display helpers for source documents (registry-backed).

use sea_orm::DatabaseConnection;

use crate::plugins::finance_accounts::source_doc_registry::SourceDocRegistry;

/// Map a stored source document type key to a display label via the registry.
pub fn source_doc_type_label(registry: &SourceDocRegistry, typ: &str) -> String {
    registry.type_display_name(typ)
}

/// Build a short summary for pickers when only type/ids are known (no instance load).
pub fn source_doc_summary(
    registry: &SourceDocRegistry,
    typ: &str,
    source_doc_id: i64,
    row_id: i64,
) -> String {
    format!(
        "{} · ref {} · #{}",
        source_doc_type_label(registry, typ),
        source_doc_id,
        row_id
    )
}

/// Build a compact summary without the source_docs row id.
pub fn source_doc_ref_summary(
    registry: &SourceDocRegistry,
    typ: &str,
    source_doc_id: i64,
) -> String {
    format!(
        "{} · ref {}",
        source_doc_type_label(registry, typ),
        source_doc_id
    )
}

/// Resolved display fields for a `source_docs` row (instance name is not persisted).
#[derive(Clone, Debug, Default)]
pub struct SourceDocDisplay {
    pub type_label: String,
    pub instance_name: String,
    pub detail_url: String,
}

impl SourceDocDisplay {
    pub fn empty() -> Self {
        Self {
            type_label: "—".into(),
            instance_name: "—".into(),
            detail_url: String::new(),
        }
    }

    /// Picker / form display: prefer type + instance when both are meaningful.
    pub fn summary_label(&self) -> String {
        if self.instance_name.is_empty() || self.instance_name == "—" {
            self.type_label.clone()
        } else if self.type_label.is_empty() || self.type_label == "—" {
            self.instance_name.clone()
        } else {
            format!("{} · {}", self.type_label, self.instance_name)
        }
    }
}

/// Load a `source_docs` row and resolve type label + instance name + detail URL.
pub async fn resolve_source_doc_display(
    db: &DatabaseConnection,
    registry: &SourceDocRegistry,
    source_docs_row_id: i64,
) -> SourceDocDisplay {
    use crate::plugins::finance_accounts::scope::load_source_doc_by_id;

    if source_docs_row_id <= 0 {
        return SourceDocDisplay::empty();
    }
    let Some(doc) = load_source_doc_by_id(db, source_docs_row_id).await else {
        return SourceDocDisplay::empty();
    };
    let type_label = registry.type_display_name(&doc.source_doc_type);
    if doc.source_doc_id <= 0 {
        return SourceDocDisplay {
            type_label,
            instance_name: "—".into(),
            detail_url: String::new(),
        };
    }
    match registry
        .resolve_instance(db, &doc.source_doc_type, doc.source_doc_id)
        .await
    {
        Ok(inst) => SourceDocDisplay {
            type_label,
            instance_name: inst.display_name(),
            detail_url: inst.detail_url(),
        },
        Err(_) => SourceDocDisplay {
            type_label,
            instance_name: "—".into(),
            detail_url: registry
                .type_detail_url(&doc.source_doc_type, doc.source_doc_id)
                .unwrap_or_default(),
        },
    }
}
