//! Request form structs for assistant test cases.

use lariv_core::html_form::{
    html_form,
    widgets::{CodeEditor, Text},
};

pub const DEFINITION_HINT: &str = "Markdown instructions given to the assistant.";
pub const SOLUTION_HINT: &str = "Rune script. main returns Result<Result>: an inner Ok passes the test case; an inner Err fails it with that error.";

pub const TEST_CASE_CREATE_FORM: &str = "p_llm_assistant_tester.TestCaseCreateForm";
pub const TEST_CASE_EDIT_FORM: &str = "p_llm_assistant_tester.TestCaseEditForm";
pub const TEST_CASE_DELETE_FORM: &str = "p_llm_assistant_tester.TestCaseDeleteForm";

#[html_form]
pub struct TestCaseForm {
    #[form(label = "Test Case Name", required, widget = Text)]
    pub name: String,

    #[form(
        label = "Test Case Definition",
        widget = CodeEditor,
        language = "markdown",
        rows = 12,
        hint = DEFINITION_HINT
    )]
    pub definition: String,

    #[form(
        label = "Test Case Solution",
        widget = CodeEditor,
        language = "plaintext",
        rows = 12,
        hint = SOLUTION_HINT
    )]
    pub solution: String,
}
