//! Preferences form for the mdBook site.

use lariv_core::html_form::{
    html_form,
    widgets::{Checkbox, Text},
};
use lariv_plugin_filesystem::routes::VNodeSelectRouteTag;

#[html_form]
pub struct PreferencesForm {
    #[form(
        label = "Source directory",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "mdbook-source-vnode",
        display = "source_vnode",
        placeholder = "Select source directory…",
        required
    )]
    pub source_vnode_id: String,

    #[form(
        label = "Output directory",
        widget = ForeignKey,
        route = VNodeSelectRouteTag,
        swap_key = "mdbook-output-vnode",
        display = "output_vnode",
        placeholder = "Select output directory…",
        required
    )]
    pub output_vnode_id: String,

    #[form(label = "URL prefix", widget = Text, required)]
    pub url_prefix: String,

    #[form(label = "Build when the server starts", widget = Checkbox)]
    pub build_on_serve: String,
}

#[html_form]
pub struct BuildForm {}

pub fn checkbox_on(raw: &str) -> bool {
    raw == "on" || raw == "true" || raw == "1"
}
