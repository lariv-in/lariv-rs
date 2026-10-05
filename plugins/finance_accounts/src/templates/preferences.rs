use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
        ButtonSubmit, Crumb, CsrfToken, FieldTitle, FormOpts, ShellChrome, breadcrumbs,
        button_submit, container_column, container_row, field_title, form, form_hx_post_main,
    };
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::routes::{
    AccountingPreferencesPostRouteTag, AccountingPreferencesRouteTag,
    FinanceAccountsPreferencesMutate,
};

use super::common::{app_scaffold, layout_main_with_crumbs, layout_with_sidebar_crumbs};

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
        let inputs = html! {
            (self.accounts_inputs)
            (self.addon_inputs)
        };
        let can_save = lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<FinanceAccountsPreferencesMutate>(
            ),
        );
        html! {
            (container_column("@container", html! {
                (field_title(FieldTitle { value: "Accounting Preferences", classes: "" }))
                @if can_save {
                    (form(&CsrfToken::current(), FormOpts {
                        attrs: form_hx_post_main(AccountingPreferencesPostRouteTag),
                        inputs: inputs,
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
                } @else {
                    fieldset disabled {
                        (inputs)
                    }
                }
            }))
        }
    }
}

impl RenderAppPane for AccountingPreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &AccountingPreferencesRouteTag.url(),
            accounting_preferences_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
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
