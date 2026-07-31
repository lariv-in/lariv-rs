//! Export catalog capability — plugins register exportable DB tables.

use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, mount_with_hooks},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for the export catalog.
pub struct ExportTag;

/// A table registered for XLSX export.
#[derive(Clone, Debug)]
pub struct ExportTable {
    pub table: String,
    pub model_name: String,
    pub columns: Vec<String>,
    pub primary_keys: Vec<String>,
    /// FK target tables pulled in when this table is selected.
    pub immediate_deps: Vec<String>,
}

impl ExportTable {
    pub fn new(
        table: impl Into<String>,
        model_name: impl Into<String>,
        columns: Vec<String>,
    ) -> Self {
        Self {
            table: table.into(),
            model_name: model_name.into(),
            columns,
            primary_keys: vec!["id".into()],
            immediate_deps: Vec::new(),
        }
    }

    pub fn with_deps(mut self, deps: Vec<String>) -> Self {
        self.immediate_deps = deps;
        self
    }

    pub fn with_primary_keys(mut self, pks: Vec<String>) -> Self {
        self.primary_keys = pks;
        self
    }
}

/// Resolved export catalog (sorted tables).
#[derive(Clone, Debug, Default)]
pub struct ExportCatalog {
    pub entries: Vec<ExportTable>,
}

impl ExportCatalog {
    pub fn entry(&self, table: &str) -> Option<&ExportTable> {
        self.entries.iter().find(|e| e.table == table)
    }
}

/// Expanded model selection (Go `ExpandedSelection`).
#[derive(Clone, Debug)]
pub struct ExpandedSelection {
    pub roots: Vec<String>,
    pub tables: Vec<String>,
}

/// Plugin hook for registering export tables.
pub trait ExportRegistrar: Sized {
    fn register_export(self, export: ExportCapability) -> ExportCapability;
}

/// Builder-phase export capability.
#[derive(Clone, Debug, Default)]
pub struct ExportCapability {
    tables: Vec<ExportTable>,
}

impl ExportCapability {
    pub fn new() -> Self {
        Self { tables: Vec::new() }
    }

    pub fn register(mut self, table: ExportTable) -> Self {
        if let Some(existing) = self.tables.iter_mut().find(|t| t.table == table.table) {
            *existing = table;
        } else {
            self.tables.push(table);
        }
        self
    }

    pub fn tables(&self) -> &[ExportTable] {
        &self.tables
    }

    pub fn catalog(&self) -> ExportCatalog {
        let mut entries = self.tables.clone();
        entries.sort_by(|a, b| a.table.cmp(&b.table));
        ExportCatalog { entries }
    }

    pub fn expand_selection(&self, roots: &[String]) -> Result<ExpandedSelection, String> {
        expand_selection(&self.catalog(), roots)
    }
}

pub fn expand_selection(catalog: &ExportCatalog, roots: &[String]) -> Result<ExpandedSelection, String> {
    let normalized = normalize_selection(roots);
    if normalized.is_empty() {
        return Err("select at least one model".into());
    }

    let mut included = std::collections::BTreeSet::new();
    let mut queue: Vec<String> = Vec::new();

    for table in &normalized {
        if catalog.entry(table).is_none() {
            return Err(format!("unknown model table {table:?}"));
        }
        if included.insert(table.clone()) {
            queue.push(table.clone());
        }
    }

    while let Some(table) = queue.first().cloned() {
        queue.remove(0);
        let Some(entry) = catalog.entry(&table) else {
            continue;
        };
        for dep in &entry.immediate_deps {
            if catalog.entry(dep).is_some() && included.insert(dep.clone()) {
                queue.push(dep.clone());
            }
        }
    }

    Ok(ExpandedSelection {
        roots: normalized,
        tables: included.into_iter().collect(),
    })
}

fn normalize_selection(tables: &[String]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for table in tables {
        if table.is_empty() || !seen.insert(table.clone()) {
            continue;
        }
        out.push(table.clone());
    }
    out.sort();
    out
}

pub type ExportCap<Hooks> = CapStore<ExportTag, Hooks, ExportCapability>;

impl<Hooks> ExportCap<Hooks> {
    pub fn resolve_hooks<Proof>(self) -> ExportCap<HNil>
    where
        Hooks: ApplyHooks<ExportCapability, Proof, Output = ExportCapability>,
    {
        CapStore::with_items(self.hooks.apply_hooks(self.items))
    }
}

impl<Plugin, H, Tail, TailProof> ApplyHooks<ExportCapability, (TailProof, ())>
    for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: ApplyHooks<ExportCapability, TailProof, Output = ExportCapability>,
    H: ExportRegistrar,
{
    type Output = ExportCapability;

    fn apply_hooks(self, items: ExportCapability) -> Self::Output {
        let items = self.tail.apply_hooks(items);
        self.head.value.register_export(items)
    }
}

impl<Hooks> Capability for ExportCap<Hooks>
where
    Hooks: ApplyHooks<ExportCapability, (), Output = ExportCapability>,
{
    type Value = ExportCapability;
    type Output = Tagged<ExportTag, ExportCapability>;
    type Hooks = Hooks;
    type Items = ExportCapability;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, |items| items)
    }
}

#[macro_export]
macro_rules! define_register_export {
    (
        plugin: $plugin:ty;
        table: $table:expr;
        model: $model:expr;
        columns: [$($col:expr),* $(,)?];
        $(deps: [$($dep:expr),* $(,)?];)?
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct ExportHook;

        impl $crate::export::ExportRegistrar for ExportHook {
            fn register_export(
                self,
                export: $crate::export::ExportCapability,
            ) -> $crate::export::ExportCapability {
                export.register(
                    $crate::export::ExportTable::new($table, $model, vec![$(::std::convert::Into::into($col)),*])
                        $(.with_deps(vec![$(::std::convert::Into::into($dep)),*]))?
                )
            }
        }
    };
}

pub use crate::define_register_export;

pub fn with_export<L, Proof>(app: App<L>) -> App<HCons<ExportCap<HNil>, L>>
where
    L: HList + CapTagAbsent<ExportTag, Proof>,
{
    app.add_capability(CapStore::with_items(ExportCapability::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_includes_deps() {
        let export = ExportCapability::new()
            .register(
                ExportTable::new("clients", "Client", vec!["id".into()])
                    .with_deps(vec!["users".into()]),
            )
            .register(ExportTable::new("users", "User", vec!["id".into()]));
        let sel = export
            .expand_selection(&["clients".into()])
            .expect("expand");
        assert_eq!(sel.tables, vec!["clients", "users"]);
    }
}
