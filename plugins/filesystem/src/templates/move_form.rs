use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, Crumb, FormOpts, ShellChrome, breadcrumbs, button_submit, form, form_hx_post_main,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::forms::{MoveForm, MoveFormField};
use crate::routes::{
    VNodeBulkMovePostRouteTag, VNodeListRouteTag, VNodeMovePostRouteTag, VNodeMoveSelectRouteTag,
};

use super::{
    app_scaffold, filesystem_item_crumbs, main_menu, scaffold_main, scaffold_pane, vnode_menu,
};

/// Move form: pick a new parent directory.
#[derive(Generic)]
pub struct VNodeMoveFormPage {
    pub id: i64,
    pub name: String,
    pub is_directory: bool,
    pub destination_id: i64,
    pub destination_display: String,
    pub error: String,
}

impl VNodeMoveFormPage {
    fn crumbs(&self) -> Markup {
        filesystem_item_crumbs(self.id, &self.name, Some("Move"))
    }

    fn pane_body(&self) -> Markup {
        let destination_id_s = if self.destination_id == 0 {
            String::new()
        } else {
            self.destination_id.to_string()
        };
        let select_url = VNodeMoveSelectRouteTag
            .with_query()
            .query("exclude_id", self.id)
            .build_with_query();
        let ctx = FormCtx::form::<MoveForm>(CsrfToken::current())
            .value(MoveFormField::DestinationId, destination_id_s.as_str())
            .display(
                MoveFormField::DestinationId,
                self.destination_display.as_str(),
            )
            .url(MoveFormField::DestinationId, select_url.as_str());
        form(
            &CsrfToken::current(),
            FormOpts {
                title: "Move Item",
                subtitle: &format!("Choose a new location for \"{}\"", self.name),
                attrs: form_hx_post_main(VNodeMovePostRouteTag::new(self.id)),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: MoveForm::render_inputs(&ctx),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Move",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for VNodeMoveFormPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            vnode_menu(self.id, &self.name, self.is_directory, "move"),
            self.crumbs(),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.pane_body())
    }
}

impl RenderTemplate for VNodeMoveFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            &format!("Move {} — Lariv", self.name),
            chrome,
            vnode_menu(self.id, &self.name, self.is_directory, "move"),
            self.crumbs(),
            self.pane_body(),
        )
    }
}

/// Bulk move form: pick a destination for many selected nodes.
#[derive(Generic)]
pub struct VNodeBulkMoveFormPage {
    pub ids: String,
    pub count: usize,
    pub destination_id: i64,
    pub destination_display: String,
    pub return_to: String,
    pub error: String,
}

impl VNodeBulkMoveFormPage {
    fn crumbs(&self) -> Markup {
        let list_url = VNodeListRouteTag.url();
        breadcrumbs(&[
            Crumb {
                label: "Filesystem",
                href: Some(&list_url),
            },
            Crumb {
                label: "Move selected",
                href: None,
            },
        ])
    }

    fn pane_body(&self) -> Markup {
        let destination_id_s = if self.destination_id == 0 {
            String::new()
        } else {
            self.destination_id.to_string()
        };
        let select_url = VNodeMoveSelectRouteTag.url();
        let ctx = FormCtx::form::<MoveForm>(CsrfToken::current())
            .value(MoveFormField::DestinationId, destination_id_s.as_str())
            .display(
                MoveFormField::DestinationId,
                self.destination_display.as_str(),
            )
            .url(MoveFormField::DestinationId, select_url.as_str());
        let subtitle = if self.count == 1 {
            "Choose a new location for the selected item".to_string()
        } else {
            format!("Choose a new location for {} selected items", self.count)
        };
        form(
            &CsrfToken::current(),
            FormOpts {
                title: "Move Selected",
                subtitle: &subtitle,
                attrs: form_hx_post_main(VNodeBulkMovePostRouteTag),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                inputs: html! {
                    input type="hidden" name="ids" value=(self.ids);
                    input type="hidden" name="return" value=(self.return_to);
                    (MoveForm::render_inputs(&ctx))
                },
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Move",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for VNodeBulkMoveFormPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            main_menu(&VNodeListRouteTag.path()),
            self.crumbs(),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self.crumbs(), self.pane_body())
    }
}

impl RenderTemplate for VNodeBulkMoveFormPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Move Selected — Lariv",
            chrome,
            main_menu(&VNodeListRouteTag.path()),
            self.crumbs(),
            self.pane_body(),
        )
    }
}
