//! Indian GST seed data, default general ledger, and default finance preferences.

pub mod migrations;

pub struct FinanceIndianTag;

lariv_core::define_plugin_install! {
    plugin: FinanceIndianTag;
    steps: [
        migrations(migrations::Hook),
    ]
}
