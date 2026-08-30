//! Request form structs for website admin.

use crate::html_form::{
    empty_str_as_none, form_checkbox_bool, form_vec_i64, html_form,
    widgets::{Checkbox, ForeignKey, Kind, ManyToMany, Select, Text},
};

const _: fn() = || {
    let _: Kind = Kind;
};

#[html_form(default)]
pub enum PageSource {
    #[form(label = "Template Page")]
    Existing {
        #[form(
            label = "Template Page",
            widget = ForeignKey,
            url = "/filesystem/file-select/",
            swap_key = "fk-website-page",
            display = "page_name",
            error = "page",
            placeholder = "Select a page file..."
        )]
        page_id: Option<i64>,
    },

    #[form(label = "Create new HTML file")]
    CreateNew {
        #[form(label = "New file name", widget = Text, error = "name")]
        new_page_name: Option<String>,
    },
}

#[html_form]
pub struct RouteCreateForm {
    #[form(label = "Path", required, widget = Text)]
    pub path: String,

    #[form(widget = Kind)]
    pub page_source: PageSource,

    #[form(
        label = "Reference Files",
        widget = ManyToMany,
        url = "/filesystem/file-select/",
        swap_key = "fk-website-refs",
        placeholder = "Select reference files..."
    )]
    pub references: Vec<i64>,

    #[form(label = "Is Active", widget = Checkbox, row = "status")]
    pub is_active: bool,

    #[form(label = "Theme", widget = Select, choices = "theme", row = "status")]
    pub theme: Option<String>,
}

#[html_form]
pub struct RouteEditForm {
    #[form(label = "Path", required, widget = Text)]
    pub path: String,

    #[form(
        label = "Template Page",
        widget = ForeignKey,
        url = "/filesystem/file-select/",
        swap_key = "fk-website-page",
        display = "page_name",
        error = "page",
        placeholder = "Select a page file..."
    )]
    pub page_id: Option<i64>,

    #[form(
        label = "Reference Files",
        widget = ManyToMany,
        url = "/filesystem/file-select/",
        swap_key = "fk-website-refs",
        placeholder = "Select reference files..."
    )]
    pub references: Vec<i64>,

    #[form(label = "Is Active", widget = Checkbox, row = "status")]
    pub is_active: bool,

    #[form(label = "Theme", widget = Select, choices = "theme", row = "status")]
    pub theme: Option<String>,
}

/// Urlencoded POST body for create (matches [`RouteCreateForm`] field names).
#[derive(Debug, serde::Deserialize)]
pub struct RouteCreateBody {
    #[serde(rename = "Path", alias = "path")]
    pub path: String,
    #[serde(rename = "Kind", alias = "kind", default)]
    pub kind: String,
    #[serde(
        rename = "PageID",
        alias = "page_id",
        default,
        deserialize_with = "empty_str_as_none"
    )]
    pub page_id: Option<i64>,
    #[serde(rename = "NewPageName", alias = "new_page_name", default)]
    pub new_page_name: Option<String>,
    #[serde(
        rename = "References",
        alias = "references",
        default,
        deserialize_with = "form_vec_i64"
    )]
    pub references: Vec<i64>,
    #[serde(
        rename = "IsActive",
        alias = "is_active",
        default,
        deserialize_with = "form_checkbox_bool"
    )]
    pub is_active: bool,
    #[serde(rename = "Theme", alias = "theme", default)]
    pub theme: Option<String>,
}

/// Urlencoded POST body for edit (matches [`RouteEditForm`] field names).
#[derive(Debug, serde::Deserialize)]
pub struct RouteEditBody {
    #[serde(rename = "Path", alias = "path")]
    pub path: String,
    #[serde(
        rename = "PageID",
        alias = "page_id",
        default,
        deserialize_with = "empty_str_as_none"
    )]
    pub page_id: Option<i64>,
    #[serde(
        rename = "References",
        alias = "references",
        default,
        deserialize_with = "form_vec_i64"
    )]
    pub references: Vec<i64>,
    #[serde(
        rename = "IsActive",
        alias = "is_active",
        default,
        deserialize_with = "form_checkbox_bool"
    )]
    pub is_active: bool,
    #[serde(rename = "Theme", alias = "theme", default)]
    pub theme: Option<String>,
}

#[html_form]
pub struct RoutePathFilterForm {
    #[form(label = "Path", widget = Text)]
    pub path: String,
}

#[html_form]
pub struct PreferencesForm {
    #[form(
        label = "Custom theme CSS",
        widget = ForeignKey,
        url = "/filesystem/file-select/",
        swap_key = "fk-website-custom-theme-css",
        display = "custom_theme_css",
        placeholder = "Select a CSS file…"
    )]
    pub custom_theme_css_vnode_id: Option<i64>,
    #[form(
        label = "Custom theme JS",
        widget = ForeignKey,
        url = "/filesystem/file-select/",
        swap_key = "fk-website-custom-theme-js",
        display = "custom_theme_js",
        placeholder = "Select a JS file…"
    )]
    pub custom_theme_js_vnode_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::{
        PageSource, RouteCreateForm, RouteCreateFormField, RouteEditForm, RouteEditFormField,
    };
    use crate::html_form::{FormCtx, HtmlForm, HtmlKind};

    #[test]
    fn page_source_kind_variants() {
        assert_eq!(PageSource::kind_tag(), "Kind");
        let v = PageSource::variants();
        assert_eq!(v[0].value, "Existing");
        assert_eq!(v[1].value, "CreateNew");
    }

    #[test]
    fn route_create_renders_kind() {
        let ctx = FormCtx::form::<RouteCreateForm>().kind::<PageSource>("Existing");
        let html = RouteCreateForm::render_inputs(&ctx).into_string();
        assert!(html.contains("type=\"radio\""), "{html}");
        assert!(html.contains("name=\"Kind\""), "{html}");
    }

    #[test]
    fn route_edit_has_no_kind_radios() {
        let ctx = FormCtx::form::<RouteEditForm>().value(RouteEditFormField::Path, "/");
        let html = RouteEditForm::render_inputs(&ctx).into_string();
        assert!(!html.contains("type=\"radio\""), "{html}");
        assert!(html.contains("name=\"PageID\""), "{html}");
    }
}
