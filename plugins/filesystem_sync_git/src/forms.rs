use lariv_core::html_form::{html_form, widgets::Text};
use lariv_plugin_filesystem::routes::VNodeSelectRouteTag;

#[html_form]
pub struct GitSyncForm {
    #[form(label = "Git URL", widget = Text, required)]
    pub git_url: String,

    #[form(label = "Git identifier", widget = Text, required)]
    pub git_identifier: String,

    #[form(label = "Subpath", widget = Text)]
    pub subpath: String,

    #[form(
        label = "Directory",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "filesystem-git-sync-directory",
        display = "directory_vnode",
        placeholder = "Select directory…",
        required
    )]
    pub directory_vnode_id: String,
}
