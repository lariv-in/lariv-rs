use frunk::Generic;
use maud::{Markup, html};

use crate::{
    components::{
        ButtonSubmit, Crumb, FieldTitle, FormOpts, ShellChrome, breadcrumbs, button_submit,
        container_column, container_row, field_title, form, form_hx_post_main,
    },
    template::{RenderAppPane, RenderTemplate},
};

use crate::plugins::finance_accounts::routes::{AccountingPreferencesPostRouteTag, AccountingPreferencesRouteTag};

use super::common::{
    app_scaffold, layout_main_with_crumbs, layout_with_sidebar_crumbs,
};

fn accounting_preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Accounting preferences",
        href: None,
    }])
}

/// Shell page: accounts-owned fields first, then addon patches.
#[derive(Generic)]
pub struct AccountingPreferencesPage {
    pub accounts_inputs: Markup,
    pub addon_inputs: Markup,
}

impl AccountingPreferencesPage {
    fn body(&self) -> Markup {
        html! {
            (container_column("@container", html! {
                (field_title(FieldTitle { value: "Accounting Preferences", classes: "" }))
                (form(FormOpts {
                    attrs: form_hx_post_main(AccountingPreferencesPostRouteTag),
                    inputs: html! {
                        (self.accounts_inputs)
                        (self.addon_inputs)
                    },
                    actions: html! {
                        (container_row("flex gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save Preferences",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                }))
            }))
        }
    }
}

impl RenderAppPane for AccountingPreferencesPage {
    fn render_pane(&self) -> crate::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &AccountingPreferencesRouteTag.url(),
            accounting_preferences_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> crate::components::MainContentHtml {
        layout_main_with_crumbs(accounting_preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for AccountingPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Accounting Preferences",
            chrome,
            accounting_preferences_crumbs(),
            self.body(),
            &AccountingPreferencesRouteTag.url(),
        )
    }
}
