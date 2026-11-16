//! "Logs" entry on the admin user-detail sidebar.

use lariv_plugin_users::user_detail_menu::{
    UserDetailLink, UserDetailMenuRegistrar, UserDetailMenuRegistry,
};

use crate::routes::UserLogListRouteTag;

fn logs_url(user_id: i64) -> String {
    UserLogListRouteTag::new(user_id).url()
}

/// Install hook that adds the Logs sidebar item.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl UserDetailMenuRegistrar for Hook {
    fn register_user_detail_links(self, cap: UserDetailMenuRegistry) -> UserDetailMenuRegistry {
        cap.push(UserDetailLink {
            key: "logs",
            title: "Logs",
            url_fn: logs_url,
            order: 20,
        })
    }
}
