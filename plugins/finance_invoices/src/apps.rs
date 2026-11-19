use lariv_core::apps::{AppTile, AppsCapability, AppsRegistrar, PluginType};

use lariv_plugin_finance_accounts::ACCOUNTING_APP_KEY;

use crate::scope::hub_tab_url;

const INVOICES_APP_KEY: &str = "p_finance_invoices";

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AppsRegistrar for Hook {
    fn register_apps(self, apps: AppsCapability) -> AppsCapability {
        let drafts = hub_tab_url("drafts");
        let apps = apps.register(AppTile {
            key: INVOICES_APP_KEY.into(),
            verbose_name: "Finance invoices".into(),
            href: drafts.clone(),
            icon: "document-text".into(),
            plugin_type: PluginType::Addon,
            roles: vec![],
        });
        patch_accounting_app_default_url(apps, drafts)
    }
}

fn patch_accounting_app_default_url(apps: AppsCapability, drafts_href: String) -> AppsCapability {
    let Some(accounting) = apps
        .apps()
        .iter()
        .find(|tile| tile.key == ACCOUNTING_APP_KEY)
        .cloned()
    else {
        return apps;
    };

    let mut patched = accounting;
    patched.href = drafts_href;
    apps.register(patched)
}
