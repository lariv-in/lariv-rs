use lariv_core::apps::{AppTile, AppsCapability, AppsRegistrar, PluginType};

use lariv_plugin_hr::HR_APP_KEY;

use crate::routes::AttendanceListRouteTag;

const ATTENDANCE_APP_KEY: &str = "p_hr_attendance";

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl AppsRegistrar for Hook {
    fn register_apps(self, apps: AppsCapability) -> AppsCapability {
        let apps = apps.register(AppTile {
            key: ATTENDANCE_APP_KEY.into(),
            verbose_name: "HR attendance".into(),
            href: AttendanceListRouteTag.url(),
            icon: "clock".into(),
            plugin_type: PluginType::Addon,
            roles: vec![],
        });
        patch_hr_app_default_url(apps)
    }
}

fn patch_hr_app_default_url(apps: AppsCapability) -> AppsCapability {
    let Some(hr) = apps
        .apps()
        .iter()
        .find(|tile| tile.key == HR_APP_KEY)
        .cloned()
    else {
        return apps;
    };

    let mut patched = hr;
    patched.href = AttendanceListRouteTag.url();
    apps.register(patched)
}
