use crate::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Datetime, ForeignKey, Text},
};
use maud::Markup;

use super::components::{InputFormAnswers, InputFormQuestions, input_form_answers, input_form_questions};
use super::routes::FormFkSelectRouteTag;

/// Custom widget for the visual question builder.
pub struct FormQuestionsDraft;
impl FormWidget for FormQuestionsDraft {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_form_questions(InputFormQuestions {
            name: field.name,
            defaults: field.value,
            classes: "w-full",
        })
    }
}

/// Custom widget for dynamic answer editing.
pub struct FormAnswersDraft;
impl FormWidget for FormAnswersDraft {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_form_answers(InputFormAnswers {
            name: field.name,
            defaults: field.value,
            questions_json: ctx.display_of("questions_json"),
            forms_catalog_json: ctx.display_of("forms_catalog_json"),
            classes: "w-full",
        })
    }
}

#[html_form]
pub struct SurveyForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Questions", required, widget = FormQuestionsDraft)]
    pub questions_json: String,

    #[form(
        label = "Created by",
        widget = ForeignKey,
        url = "/users/select/",
        swap_key = "fk-form-author",
        display = "author",
        required,
        placeholder = "Select author…"
    )]
    pub created_by_id: i64,
}

#[html_form]
pub struct SurveyFilterForm {
    #[form(label = "Title", widget = Text)]
    pub title: String,
}

#[html_form]
pub struct FormResponseForm {
    #[form(
        label = "Form",
        required,
        widget = ForeignKey,
        route = FormFkSelectRouteTag,
        swap_key = "fk-form-response",
        display = "form",
        placeholder = "Select form…"
    )]
    pub form_id: i64,

    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,

    #[form(label = "Submitted at", required, widget = Datetime)]
    pub submitted_at: String,

    #[form(
        label = "Answers",
        required,
        widget = FormAnswersDraft,
        display = "questions_json"
    )]
    pub answers_json: String,
}

#[html_form]
pub struct FormResponseScopedFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,

    #[form(label = "Email", widget = Text)]
    pub email: String,
}

