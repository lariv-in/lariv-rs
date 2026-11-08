//! Preferences page for the mdBook site.

use frunk::Generic;
use maud::{Markup, html};

use lariv_core::capability::define_register_items;
use lariv_core::components::{
    ButtonSubmit, Crumb, FormOpts, LayoutMain, LayoutSidebar, ShellChrome, ShellScaffold,
    SidebarMenu, SidebarNavLink, breadcrumbs, button_submit, form, form_hx_post_main, layout_main,
    layout_sidebar, shell_scaffold, sidebar_menu, sidebar_nav_items_pane,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};

use super::forms::{BuildForm, PreferencesForm, PreferencesFormField};
use super::routes::{MdBookBuildPostRouteTag, MdBookPrefsGetRouteTag, MdBookPrefsPostRouteTag};

#[derive(Generic)]
pub struct MdBookPreferencesPage {
    pub source_vnode_id: String,
    pub source_display: String,
    pub output_vnode_id: String,
    pub output_display: String,
    pub url_prefix: String,
    pub build_on_serve: bool,
    pub error: String,
    pub status: String,
}

impl MdBookPreferencesPage {
    fn body(&self) -> Markup {
        html! {
            @if !self.status.is_empty() {
                p class="mb-4" { (self.status) }
            }
            (form(
                &CsrfToken::current(),
                FormOpts {
                    attrs: form_hx_post_main(MdBookPrefsPostRouteTag),
                    title: "mdBook",
                    subtitle: "Choose filesystem directories for the book source and the built site.",
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: PreferencesForm::render_inputs(
                        &FormCtx::form::<PreferencesForm>(CsrfToken::current())
                            .value(
                                PreferencesFormField::SourceVnodeId,
                                self.source_vnode_id.as_str(),
                            )
                            .display(
                                PreferencesFormField::SourceVnodeId,
                                self.source_display.as_str(),
                            )
                            .value(
                                PreferencesFormField::OutputVnodeId,
                                self.output_vnode_id.as_str(),
                            )
                            .display(
                                PreferencesFormField::OutputVnodeId,
                                self.output_display.as_str(),
                            )
                            .value(PreferencesFormField::UrlPrefix, self.url_prefix.as_str())
                            .checked(PreferencesFormField::BuildOnServe, self.build_on_serve),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit {
                            label: "Save preferences",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                },
            ))
            div class="mt-6" {
                (form(
                    &CsrfToken::current(),
                    FormOpts {
                        attrs: form_hx_post_main(MdBookBuildPostRouteTag),
                        title: "Publish",
                        subtitle: "Build the saved source directory and replace the website routes.",
                        inputs: BuildForm::render_inputs(
                            &FormCtx::form::<BuildForm>(CsrfToken::current()),
                        ),
                        actions: html! {
                            (button_submit(ButtonSubmit {
                                label: "Build site",
                                ..Default::default()
                            }))
                        },
                        ..Default::default()
                    },
                ))
            }
        }
    }
}

fn crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "mdBook",
        href: None,
    }])
}

fn menu(current_path: &str) -> Markup {
    let prefs_url = MdBookPrefsGetRouteTag.url();
    let links = [SidebarNavLink {
        key: "preferences",
        title: "Preferences",
        url: &prefs_url,
        icon_name: None,
        match_prefixes: &[],
    }];
    sidebar_menu(SidebarMenu {
        title: "mdBook",
        children: sidebar_nav_items_pane(&links, current_path),
    })
}

impl lariv_core::template::RenderAppPane for MdBookPreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_sidebar(LayoutSidebar {
            sidebar: menu(&MdBookPrefsGetRouteTag.url()),
            breadcrumbs: crumbs(),
            content: self.body(),
        })
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main(LayoutMain {
            breadcrumbs: crumbs(),
            content: self.body(),
        })
    }
}

impl RenderTemplate for MdBookPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        shell_scaffold(ShellScaffold {
            title: "Lariv",
            registry_head: chrome.head.clone(),
            topbar_items: chrome.topbar_items.clone(),
            right_sidebar: chrome.right_sidebar.clone(),
            sidebar: menu(&MdBookPrefsGetRouteTag.url()),
            breadcrumbs: crumbs(),
            body: self.body(),
            ..Default::default()
        })
    }
}

define_register_items! {
    plugin: MdBookTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        MdBookPreferencesIdx: MdBookPreferencesPageTag => MdBookPreferencesPage,
    ]
}
