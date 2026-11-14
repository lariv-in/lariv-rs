use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, container_row, form, form_hx_post_url,
    modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::{CreateModal, modal_create_post_url};

use crate::forms::{TEST_CASE_CREATE_FORM, TestCaseForm, TestCaseFormField};
use crate::keys::TEST_CASE_CREATE_MODAL;
use crate::routes::{TestCaseCreateGetRouteTag, TestCaseCreatePostRouteTag};

pub const TEST_CASE_CREATE: CreateModal<TestCaseCreateGetRouteTag, TestCaseCreatePostRouteTag> =
    CreateModal {
        key: TEST_CASE_CREATE_MODAL,
        form_name: TEST_CASE_CREATE_FORM,
        get: TestCaseCreateGetRouteTag,
        post: TestCaseCreatePostRouteTag,
    };

#[derive(Generic)]
pub struct TestCaseCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub name: String,
    pub definition: String,
    pub solution: String,
    pub error: String,
}

impl RenderTemplate for TestCaseCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            TEST_CASE_CREATE_FORM
        } else {
            self.form_name.as_str()
        };
        let ctx = FormCtx::form::<TestCaseForm>(CsrfToken::current())
            .value(TestCaseFormField::Name, self.name.as_str())
            .value(TestCaseFormField::Definition, self.definition.as_str())
            .value(TestCaseFormField::Solution, self.solution.as_str());
        modal_keyed(
            TEST_CASE_CREATE_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Create Test Case",
                    subtitle: "Definition is the markdown given to the assistant. Solution is the Rune check.",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        TEST_CASE_CREATE_MODAL,
                        &modal_create_post_url(
                            TestCaseCreatePostRouteTag,
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TestCaseForm::render_inputs(&ctx),
                    actions: html! {
                        (container_row(
                            "flex justify-end gap-2 mt-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save Test Case",
                                    classes: "btn-primary",
                                    ..Default::default()
                                }))
                            },
                        ))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}
