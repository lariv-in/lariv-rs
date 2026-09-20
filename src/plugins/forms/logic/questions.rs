use crate::plugins::forms::types::{FormQuestion, FormQuestionType, FormQuestions};

/// Serialize questions for hidden form fields and Alpine defaults.
pub fn questions_to_json(questions: &FormQuestions) -> String {
    serde_json::to_string(questions).unwrap_or_else(|_| "[]".into())
}

/// Parse and validate questions JSON from the admin question builder.
pub fn parse_questions_json(raw: &str) -> Result<FormQuestions, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(FormQuestions::default());
    }
    let questions: FormQuestions =
        serde_json::from_str(trimmed).map_err(|e| format!("Invalid questions JSON: {e}"))?;
    validate_questions(&questions)?;
    Ok(questions)
}

fn validate_questions(questions: &FormQuestions) -> Result<(), String> {
    for (idx, q) in questions.iter().enumerate() {
        let n = idx + 1;
        if q.display_text.trim().is_empty() {
            return Err(format!("Question {n}: display text is required"));
        }
        match &q.question_type {
            FormQuestionType::MCQText(opts)
            | FormQuestionType::MCQWithCustom(opts)
            | FormQuestionType::Checkboxes(opts)
            | FormQuestionType::Dropdown(opts) => {
                if opts.is_empty() {
                    return Err(format!(
                        "Question {n}: at least one option is required for this type"
                    ));
                }
                if opts.iter().any(|o| o.trim().is_empty()) {
                    return Err(format!("Question {n}: options cannot be empty"));
                }
            }
            FormQuestionType::LinearScale(spec) => {
                if spec.start >= spec.end {
                    return Err(format!(
                        "Question {n}: scale end must be greater than start"
                    ));
                }
            }
            FormQuestionType::Rating(spec) => {
                if spec.range == 0 {
                    return Err(format!("Question {n}: rating range must be at least 1"));
                }
            }
            FormQuestionType::MCQGrid(labels) | FormQuestionType::TickBoxGrid(labels) => {
                if labels.rows.is_empty() || labels.cols.is_empty() {
                    return Err(format!("Question {n}: grid rows and columns are required"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Human-readable summary for list/detail views.
pub fn question_type_label(qt: &FormQuestionType) -> &'static str {
    match qt {
        FormQuestionType::ShortText => "Short text",
        FormQuestionType::LongText => "Long text",
        FormQuestionType::Number(_) => "Number",
        FormQuestionType::MCQText(_) => "Multiple choice",
        FormQuestionType::MCQWithCustom(_) => "Multiple choice (custom)",
        FormQuestionType::Checkboxes(_) => "Checkboxes",
        FormQuestionType::Dropdown(_) => "Dropdown",
        FormQuestionType::LinearScale(_) => "Linear scale",
        FormQuestionType::Rating(_) => "Rating",
        FormQuestionType::MCQGrid(_) => "MCQ grid",
        FormQuestionType::TickBoxGrid(_) => "Tick-box grid",
        FormQuestionType::Date => "Date",
        FormQuestionType::Time => "Time",
    }
}

/// Convert stored questions into editor-friendly JSON (same as storage format).
pub fn questions_editor_json(questions: &FormQuestions) -> String {
    questions_to_json(questions)
}

/// Build a default empty question for the editor.
pub fn default_question() -> FormQuestion {
    FormQuestion::new("New question", FormQuestionType::ShortText, false)
}
