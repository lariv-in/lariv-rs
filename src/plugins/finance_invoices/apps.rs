use crate::apps::{AppTile, AppsCapability, AppsRegistrar, PluginType};

use crate::plugins::finance_accounts::ACCOUNTING_APP_KEY;

use crate::plugins::finance_invoices::routes::InvoiceDefaultRouteTag;

const INVOICES_APP_KEY: &str = "p_finance_invoices";

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AppsRegistrar for Hook {
    fn register_apps(self, apps: AppsCapability) -> AppsCapability {
        let apps = apps.register(AppTile {
            key: INVOICES_APP_KEY.into(),
            verbose_name: "Finance invoices".into(),
            href: InvoiceDefaultRouteTag.url(),
            icon: "document-text".into(),
            plugin_type: PluginType::Addon,
            roles: vec!["superuser".into()],
        });
        patch_accounting_app_default_url(apps)
    }
}

fn patch_accounting_app_default_url(apps: AppsCapability) -> AppsCapability {
    let Some(accounting) = apps
        .apps()
        .iter()
        .find(|tile| tile.key == ACCOUNTING_APP_KEY)
        .cloned()
    else {
        return apps;
    };

    let mut patched = accounting;
    patched.href = InvoiceDefaultRouteTag.url();
    apps.register(patched)
}
