use lariv_core::apps::{AppTile, AppsCapability, AppsRegistrar, PluginType};

use crate::routes::PurchaseDefaultRouteTag;

const PURCHASES_APP_KEY: &str = "p_finance_purchases";

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AppsRegistrar for Hook {
    fn register_apps(self, apps: AppsCapability) -> AppsCapability {
        apps.register(AppTile {
            key: PURCHASES_APP_KEY.into(),
            verbose_name: "Finance purchases".into(),
            href: PurchaseDefaultRouteTag.url(),
            icon: "document-text".into(),
            plugin_type: PluginType::Addon,
            roles: vec![],
        })
    }
}
