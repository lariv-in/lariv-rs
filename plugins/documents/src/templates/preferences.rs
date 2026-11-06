use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, form, form_hx_post_main,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::forms::{PreferencesForm, PreferencesFormField};
use crate::routes::{DocumentPrefsGetRouteTag, DocumentPrefsPostRouteTag};

use super::{app_scaffold, document_menu, preferences_crumbs, scaffold_main, scaffold_pane};

#[derive(Generic)]
pub struct DocumentPreferencesPage {
    pub signing_authority_name: String,
    pub validity_duration: String,
    pub error: String,
}

impl DocumentPreferencesPage {
    fn body(&self) -> Markup {
        form(
            &CsrfToken::current(),
            FormOpts {
                attrs: form_hx_post_main(DocumentPrefsPostRouteTag),
                title: "Documents Preferences",
                subtitle: "Used as the name and lifetime of the certificate that signs a PDF",
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs: PreferencesForm::render_inputs(
                    &FormCtx::form::<PreferencesForm>(CsrfToken::current())
                        .value(
                            PreferencesFormField::SigningAuthorityName,
                            self.signing_authority_name.as_str(),
                        )
                        .value(
                            PreferencesFormField::ValidityDuration,
                            self.validity_duration.as_str(),
                        ),
                ),
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Save Preferences",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for DocumentPreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            document_menu(&DocumentPrefsGetRouteTag.url()),
            preferences_crumbs(),
            self.body(),
        )
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for DocumentPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Documents Preferences — Lariv",
            chrome,
            document_menu(&DocumentPrefsGetRouteTag.url()),
            preferences_crumbs(),
            self.body(),
        )
    }
}
