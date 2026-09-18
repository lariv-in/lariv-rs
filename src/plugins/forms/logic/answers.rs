use std::collections::HashMap;

use crate::plugins::forms::types::{
    FormAnswer, FormAnswers, FormQuestionId, FormQuestionType, FormQuestions,
};

/// Serialize answers for hidden form fields.
pub fn answers_to_json(answers: &FormAnswers) -> String {
    serde_json::to_string(answers).unwrap_or_else(|_| "{}".into())
}

/// Re-serialize answer JSON for safe round-trip through form fields and Alpine.
pub fn normalize_answers_json(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "{}".into();
    }
    serde_json::from_str::<serde_json::Value>(trimmed)
        .map(sanitize_answers_value)
        .and_then(|v| serde_json::to_string(&v))
        .unwrap_or_else(|_| "{}".into())
}

/// Drop unanswered entries (empty strings, empty arrays, empty grids) before deserializing.
fn sanitize_answers_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .filter(|(_, answer)| !is_empty_answer(answer))
                .collect(),
        ),
        other => other,
    }
}

fn is_empty_answer(answer: &serde_json::Value) -> bool {
    let Some(obj) = answer.as_object() else {
        return false;
    };
    if obj.len() != 1 {
        return false;
    }
    let (kind, value) = obj.iter().next().expect("len checked");
    match kind.as_str() {
        "ShortText" | "LongText" | "MCQText" | "MCQWithCustom" | "Dropdown" | "Date" | "Time" => {
            value.as_str().is_some_and(str::is_empty)
        }
        "Checkboxes" => value.as_array().is_some_and(Vec::is_empty),
        "MCQGrid" | "TickBoxGrid" => value
            .get("selections")
            .and_then(|v| v.as_object())
            .is_some_and(serde_json::Map::is_empty),
        _ => false,
    }
}

/// Parse and validate answers JSON against a form's question schema.
pub fn parse_answers_json(raw: &str, questions: &FormQuestions) -> Result<FormAnswers, String> {
    let trimmed = raw.trim();
    let answers: FormAnswers = if trimmed.is_empty() || trimmed == "{}" {
        FormAnswers::default()
    } else {
        let value = serde_json::from_str::<serde_json::Value>(trimmed)
            .map_err(|e| format!("Invalid answers JSON: {e}"))?;
        serde_json::from_value(sanitize_answers_value(value))
            .map_err(|e| format!("Invalid answers JSON: {e}"))?
    };
    validate_answers(&answers, questions)?;
    Ok(answers)
}

fn validate_answers(answers: &FormAnswers, questions: &FormQuestions) -> Result<(), String> {
    for q in questions.iter() {
        let answer = answers.get(&q.form_question_id);
        if q.required && answer.is_none() {
            return Err(format!(
                "Answer required for question: {}",
                q.display_text
            ));
        }
        if let Some(ans) = answer {
            if !answer_matches_type(ans, &q.question_type) {
                return Err(format!(
                    "Answer type mismatch for question: {}",
                    q.display_text
                ));
            }
        }
    }
    Ok(())
}

fn answer_matches_type(answer: &FormAnswer, qt: &FormQuestionType) -> bool {
    matches!(
        (answer, qt),
        (FormAnswer::ShortText(_), FormQuestionType::ShortText)
            | (FormAnswer::LongText(_), FormQuestionType::LongText)
            | (FormAnswer::Number(_), FormQuestionType::Number(_))
            | (FormAnswer::MCQText(_), FormQuestionType::MCQText(_))
            | (FormAnswer::MCQWithCustom(_), FormQuestionType::MCQWithCustom(_))
            | (FormAnswer::Checkboxes(_), FormQuestionType::Checkboxes(_))
            | (FormAnswer::Dropdown(_), FormQuestionType::Dropdown(_))
            | (FormAnswer::LinearScale(_), FormQuestionType::LinearScale(_))
            | (FormAnswer::Rating(_), FormQuestionType::Rating(_))
            | (FormAnswer::MCQGrid(_), FormQuestionType::MCQGrid(_))
            | (FormAnswer::TickBoxGrid(_), FormQuestionType::TickBoxGrid(_))
            | (FormAnswer::Date(_), FormQuestionType::Date)
            | (FormAnswer::Time(_), FormQuestionType::Time)
    )
}

/// Format a single answer for read-only display.
pub fn format_answer(answer: &FormAnswer) -> String {
    match answer {
        FormAnswer::ShortText(s) | FormAnswer::LongText(s) => s.clone(),
        FormAnswer::Number(n) => n.to_string(),
        FormAnswer::MCQText(s) | FormAnswer::MCQWithCustom(s) | FormAnswer::Dropdown(s) => {
            s.clone()
        }
        FormAnswer::Checkboxes(items) => items.join(", "),
        FormAnswer::LinearScale(n) => n.to_string(),
        FormAnswer::Rating(n) => n.to_string(),
        FormAnswer::MCQGrid(g) => g
            .selections
            .iter()
            .map(|(row, col)| format!("{row}: {col}"))
            .collect::<Vec<_>>()
            .join("; "),
        FormAnswer::TickBoxGrid(g) => g
            .selections
            .iter()
            .map(|(row, cols)| format!("{row}: {}", cols.join(", ")))
            .collect::<Vec<_>>()
            .join("; "),
        FormAnswer::Date(d) => d.to_string(),
        FormAnswer::Time(t) => t.format("%H:%M:%S").to_string(),
    }
}

/// Build empty answers map.
pub fn empty_answers() -> FormAnswers {
    FormAnswers(HashMap::new())
}

#[cfg(test)]
mod parse_tests {
    use super::{normalize_answers_json, parse_answers_json};
    use crate::plugins::forms::types::{FormAnswer, FormAnswers, FormQuestions};

    #[test]
    fn parse_full_submitted_answers_payload() {
        let json = r#"{"0f348ae0-c49c-40ab-94f3-c69718633828":{"LongText":""},"1866556c-3724-4636-89e8-f992fe25762f":{"Number":0},"555d45ce-2774-4d43-812b-f39bd4b39e9c":{"Dropdown":""},"6163346d-ec80-4515-b9a9-ef86043d0cfb":{"Checkboxes":[]},"75be0707-1412-4619-bc37-0ef1b725dc5b":{"Rating":0},"7c807514-0b8e-4b06-a6f4-28945861434b":{"Rating":0},"88010e03-055d-4247-a6b1-9ad34d2648f7":{"MCQText":""},"8ba448ed-9327-4f24-81e4-ef4ab932f2fd":{"Date":""},"9f9d6939-41f4-4d8f-91b6-3a89e97003f4":{"Number":0},"a3d86e84-18d5-42e6-82fa-a9ff77c708bd":{"Time":""},"a4464514-5ba3-454f-bbe8-d346e062813d":{"TickBoxGrid":{"selections":{"Feature X":["Good"],"Feature Y":["Good"]}}},"b0d9888b-86fd-41d1-a9f7-842e8a5165d0":{"Rating":0},"c7f3bf24-1103-421e-98ba-5a50352ecb80":{"MCQGrid":{"selections":{"Row A":"Col 1"}}},"d1e1e986-42b0-4ced-b95c-4ee7bc67bfa7":{"MCQWithCustom":""},"d27a70cd-9d9e-4d0d-a9e3-92f31f8fef1b":{"ShortText":""},"e486740a-0a1f-47d8-afb3-18f9dda1b5f8":{"LinearScale":1}}"#;
        let answers = parse_answers_json(json, &FormQuestions::default())
            .expect("payload with empty strings stripped");
        // Keeps grids, linear scale, and numeric ratings — not empty strings/arrays.
        assert_eq!(answers.len(), 8);
        assert!(!answers
            .values()
            .any(|a| matches!(a, FormAnswer::ShortText(s) if s.is_empty())));
    }

    #[test]
    fn empty_string_answers_are_stripped() {
        let json = r#"{"11111111-1111-1111-1111-111111111111":{"ShortText":""},"22222222-2222-2222-2222-222222222222":{"MCQText":"Blue"}}"#;
        let normalized = normalize_answers_json(json);
        assert!(!normalized.contains("11111111"));
        assert!(normalized.contains("Blue"));
    }
}

/// Lookup question text by id for detail display.
pub fn question_label(questions: &FormQuestions, id: &FormQuestionId) -> String {
    questions
        .iter()
        .find(|q| q.form_question_id == *id)
        .map(|q| q.display_text.clone())
        .unwrap_or_else(|| id.to_string())
}
