use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ShellChrome, button_modal_form,
    button_submit, container_row, delete_confirmation, form, form_hx_post_selector,
    form_hx_post_url, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_edit_post_url;

use crate::forms::{TEST_CASE_DELETE_FORM, TestCaseForm, TestCaseFormField};
use crate::keys::{TEST_CASE_DELETE_MODAL, TEST_CASE_EDIT_MODAL};
use crate::routes::{
    TestCaseDeleteGetRouteTag, TestCaseDeletePostRouteTag, TestCaseEditPostRouteTag,
};

/// Edit test case modal. Create uses [`super::create::TestCaseCreateModalPage`].
#[derive(Generic)]
pub struct TestCaseEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub definition: String,
    pub solution: String,
    pub error: String,
}

impl RenderTemplate for TestCaseEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = TestCaseDeleteGetRouteTag::new(self.id).url();
        let delete_post = TestCaseDeletePostRouteTag::new(self.id).path();
        let ctx = FormCtx::form::<TestCaseForm>(CsrfToken::current())
            .value(TestCaseFormField::Name, self.name.as_str())
            .value(TestCaseFormField::Definition, self.definition.as_str())
            .value(TestCaseFormField::Solution, self.solution.as_str());
        modal_keyed(
            TEST_CASE_EDIT_MODAL,
            "",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Edit Test Case",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        TEST_CASE_EDIT_MODAL,
                        &modal_edit_post_url(
                            TestCaseEditPostRouteTag::new(self.id),
                            &self.form_name,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: TestCaseForm::render_inputs(&ctx),
                    actions: html! {
                        (container_row(
                            "flex justify-end gap-2 mt-2",
                            html! {
                                (button_submit(ButtonSubmit {
                                    label: "Save",
                                    classes: "btn-primary",
                                    ..Default::default()
                                }))
                                (button_modal_form(ButtonModalForm {
                                    label: "Delete",
                                    icon_name: Some("trash"),
                                    name: TEST_CASE_DELETE_FORM,
                                    href: &delete_url,
                                    form_post_url: &delete_post,
                                    modal_uid: TEST_CASE_DELETE_MODAL.id(),
                                    classes: "btn-error",
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

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let post_url = TestCaseDeletePostRouteTag::new(self.id).url();
        modal_keyed(
            TEST_CASE_DELETE_MODAL,
            "",
            delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: "Are you sure you want to delete this test case?",
                attrs: form_hx_post_selector(&post_url, &TEST_CASE_DELETE_MODAL.selector()),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
        )
    }
}
