//! Job posting questions and applicant answers stored as Postgres composite arrays.
//!
//! SeaORM's `Value` enum cannot bind a composite array, so reads decode through sqlx
//! and writes use a `ROW(...)::type` statement on the same connection.

use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, ValueType, ValueTypeErr};
use sea_orm::{ColIdx, ConnectionTrait, DbErr, Statement, TryGetError, TryGetable};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgHasArrayType;
use sqlx::types::Json;
use uuid::Uuid;

use lariv_plugin_forms::logic::answers::format_answer;
use lariv_plugin_forms::logic::questions::{question_type_label, questions_to_json};
use lariv_plugin_forms::types::{
    FormAnswer, FormQuestion, FormQuestionId, FormQuestionType, FormQuestions, GridLabels,
    LinearScaleSpec, NumberRange, RatingSpec,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "hr_job_posting_question")]
pub struct JobPostingQuestion {
    pub job_posting_question_id: Uuid,
    pub display_text: String,
    pub question_type: Json<FormQuestionType>,
    pub required: bool,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "hr_job_posting_answer")]
pub struct JobPostingAnswer {
    pub question: JobPostingQuestion,
    pub answer: Json<FormAnswer>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobPostingQuestions(pub Vec<JobPostingQuestion>);

impl Deref for JobPostingQuestions {
    type Target = Vec<JobPostingQuestion>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JobPostingQuestions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobPostingAnswers(pub Vec<JobPostingAnswer>);

impl Deref for JobPostingAnswers {
    type Target = Vec<JobPostingAnswer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for JobPostingAnswers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn decode_vec<T, I>(res: &QueryResult, index: I) -> Result<Vec<T>, TryGetError>
where
    T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + PgHasArrayType,
    I: ColIdx,
{
    let row = res.try_as_pg_row().ok_or_else(|| {
        TryGetError::DbErr(DbErr::Custom(
            "composite arrays require a postgres row".into(),
        ))
    })?;
    let decoded = if let Some(name) = index.as_str() {
        row.try_get(name)
    } else if let Some(idx) = index.as_usize() {
        row.try_get(*idx)
    } else {
        return Err(TryGetError::DbErr(DbErr::Custom(
            "unsupported column index".into(),
        )));
    };
    decoded.map_err(|err| TryGetError::DbErr(DbErr::Custom(err.to_string())))
}

macro_rules! impl_composite_array_column {
    ($wrapper:ty, $elem:ty, $sql_name:literal) => {
        impl TryGetable for $wrapper {
            fn try_get_by<I: ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
                Ok(Self(decode_vec::<$elem, _>(res, index)?))
            }
        }

        impl From<$wrapper> for Value {
            fn from(source: $wrapper) -> Self {
                Value::Json(serde_json::to_value(source).ok().map(Box::new))
            }
        }

        impl ValueType for $wrapper {
            fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
                match v {
                    Value::Json(Some(json)) => {
                        serde_json::from_value(*json).map_err(|_| ValueTypeErr)
                    }
                    _ => Err(ValueTypeErr),
                }
            }

            fn type_name() -> String {
                $sql_name.to_owned()
            }

            fn array_type() -> ArrayType {
                ArrayType::Json
            }

            fn column_type() -> ColumnType {
                ColumnType::Array(std::sync::Arc::new(ColumnType::custom($sql_name)))
            }
        }

        impl Nullable for $wrapper {
            fn null() -> Value {
                Value::Json(None)
            }
        }
    };
}

impl_composite_array_column!(
    JobPostingQuestions,
    JobPostingQuestion,
    "hr_job_posting_question"
);
impl_composite_array_column!(JobPostingAnswers, JobPostingAnswer, "hr_job_posting_answer");

/// JSON the forms question builder already understands.
pub fn questions_editor_json(questions: &JobPostingQuestions) -> String {
    let form = FormQuestions(
        questions
            .iter()
            .map(|q| FormQuestion {
                form_question_id: FormQuestionId(q.job_posting_question_id),
                display_text: q.display_text.clone(),
                question_type: q.question_type.0.clone(),
                required: q.required,
                description: if q.description.is_empty() {
                    None
                } else {
                    Some(q.description.clone())
                },
            })
            .collect(),
    );
    questions_to_json(&form)
}

/// Parse the forms question builder payload into composite rows.
pub fn parse_questions_json(raw: &str) -> Result<JobPostingQuestions, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(JobPostingQuestions::default());
    }
    let questions: FormQuestions =
        serde_json::from_str(trimmed).map_err(|e| format!("Invalid questions JSON: {e}"))?;
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(questions.len());
    for (idx, q) in questions.0.into_iter().enumerate() {
        let n = idx + 1;
        let display_text = q.display_text.trim().to_string();
        if display_text.is_empty() {
            return Err(format!("Question {n}: display text is required"));
        }
        if !seen.insert(q.form_question_id.0) {
            return Err(format!("Question {n}: duplicate question id"));
        }
        validate_question_type(n, &q.question_type)?;
        out.push(JobPostingQuestion {
            job_posting_question_id: q.form_question_id.0,
            display_text,
            question_type: Json(q.question_type),
            required: q.required,
            description: q.description.unwrap_or_default().trim().to_string(),
        });
    }
    Ok(JobPostingQuestions(out))
}

fn validate_question_type(n: usize, qt: &FormQuestionType) -> Result<(), String> {
    match qt {
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
        FormQuestionType::ShortText
        | FormQuestionType::LongText
        | FormQuestionType::Number(_)
        | FormQuestionType::Date
        | FormQuestionType::Time => {}
    }
    Ok(())
}

/// Parse the forms answer widget payload and snapshot each answered question.
pub fn parse_answers_json(
    raw: &str,
    questions: &JobPostingQuestions,
) -> Result<JobPostingAnswers, String> {
    let trimmed = raw.trim();
    let value = if trimmed.is_empty() || trimmed == "{}" {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_str(trimmed).map_err(|e| format!("Invalid answers JSON: {e}"))?
    };
    let obj = value
        .as_object()
        .ok_or_else(|| "Invalid answers JSON: expected an object".to_string())?;
    let known: HashMap<Uuid, &JobPostingQuestion> = questions
        .iter()
        .map(|q| (q.job_posting_question_id, q))
        .collect();
    for key in obj.keys() {
        let id =
            Uuid::parse_str(key).map_err(|_| format!("Answer key is not a question id: {key}"))?;
        if !known.contains_key(&id) {
            return Err(format!("Answer does not match a posting question: {key}"));
        }
    }
    let mut out = Vec::new();
    for q in questions.iter() {
        let key = q.job_posting_question_id.to_string();
        let raw_answer = obj.get(&key);
        let kept = match raw_answer {
            Some(value) if !is_empty_answer(value) => Some(value),
            _ => None,
        };
        if kept.is_none() {
            if q.required {
                return Err(format!("Answer required for question: {}", q.display_text));
            }
            continue;
        }
        let answer: FormAnswer = serde_json::from_value(kept.expect("kept").clone())
            .map_err(|e| format!("Invalid answer for {}: {e}", q.display_text))?;
        if !answer_matches(&answer, &q.question_type.0) {
            return Err(format!(
                "Answer type mismatch for question: {}",
                q.display_text
            ));
        }
        validate_answer_bounds(&q.display_text, &answer, &q.question_type.0)?;
        out.push(JobPostingAnswer {
            question: q.clone(),
            answer: Json(answer),
        });
    }
    Ok(JobPostingAnswers(out))
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

fn answer_matches(answer: &FormAnswer, qt: &FormQuestionType) -> bool {
    matches!(
        (answer, qt),
        (FormAnswer::ShortText(_), FormQuestionType::ShortText)
            | (FormAnswer::LongText(_), FormQuestionType::LongText)
            | (FormAnswer::Number(_), FormQuestionType::Number(_))
            | (FormAnswer::MCQText(_), FormQuestionType::MCQText(_))
            | (
                FormAnswer::MCQWithCustom(_),
                FormQuestionType::MCQWithCustom(_)
            )
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

fn validate_answer_bounds(
    label: &str,
    answer: &FormAnswer,
    qt: &FormQuestionType,
) -> Result<(), String> {
    match (answer, qt) {
        (FormAnswer::Number(n), FormQuestionType::Number(NumberRange { start, end })) => {
            if start.is_some_and(|lo| *n < lo) || end.is_some_and(|hi| *n > hi) {
                return Err(format!("Number out of range for question: {label}"));
            }
        }
        (FormAnswer::MCQText(choice), FormQuestionType::MCQText(opts))
        | (FormAnswer::Dropdown(choice), FormQuestionType::Dropdown(opts)) => {
            if !opts.iter().any(|opt| opt == choice) {
                return Err(format!("Choice is not an option for question: {label}"));
            }
        }
        (FormAnswer::MCQWithCustom(choice), FormQuestionType::MCQWithCustom(_)) => {
            if choice.trim().is_empty() {
                return Err(format!("Answer required for question: {label}"));
            }
        }
        (FormAnswer::Checkboxes(selected), FormQuestionType::Checkboxes(opts)) => {
            if selected
                .iter()
                .any(|item| !opts.iter().any(|opt| opt == item))
            {
                return Err(format!("Choice is not an option for question: {label}"));
            }
        }
        (
            FormAnswer::LinearScale(n),
            FormQuestionType::LinearScale(LinearScaleSpec { start, end, .. }),
        ) => {
            if *n < *start || *n > *end {
                return Err(format!("Scale value out of range for question: {label}"));
            }
        }
        (FormAnswer::Rating(n), FormQuestionType::Rating(RatingSpec { range, .. })) => {
            if *n == 0 || *n > *range {
                return Err(format!("Rating out of range for question: {label}"));
            }
        }
        (FormAnswer::MCQGrid(grid), FormQuestionType::MCQGrid(labels)) => {
            check_grid_keys(label, &grid.selections, labels, false)?;
        }
        (FormAnswer::TickBoxGrid(grid), FormQuestionType::TickBoxGrid(labels)) => {
            for (row, cols) in &grid.selections {
                if !labels.rows.iter().any(|known| known == row) {
                    return Err(format!("Grid row is not declared for question: {label}"));
                }
                if cols
                    .iter()
                    .any(|col| !labels.cols.iter().any(|known| known == col))
                {
                    return Err(format!("Grid column is not declared for question: {label}"));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_grid_keys(
    label: &str,
    selections: &HashMap<String, String>,
    labels: &GridLabels,
    _tick: bool,
) -> Result<(), String> {
    for (row, col) in selections {
        if !labels.rows.iter().any(|known| known == row) {
            return Err(format!("Grid row is not declared for question: {label}"));
        }
        if !labels.cols.iter().any(|known| known == col) {
            return Err(format!("Grid column is not declared for question: {label}"));
        }
    }
    Ok(())
}

#[derive(frunk::Generic)]
pub struct RenderedAnswer {
    pub display_text: String,
    pub description: String,
    pub type_label: String,
    pub answer: String,
}

/// Render stored answers from the question copied onto each answer.
pub fn render_answers(answers: &JobPostingAnswers) -> Vec<RenderedAnswer> {
    answers
        .iter()
        .map(|row| RenderedAnswer {
            display_text: row.question.display_text.clone(),
            description: row.question.description.clone(),
            type_label: question_type_label(&row.question.question_type.0).to_string(),
            answer: format_answer(&row.answer.0),
        })
        .collect()
}

fn json_value<T: Serialize>(value: &T) -> Value {
    Value::Json(serde_json::to_value(value).ok().map(Box::new))
}

/// Replace `hr_job_forms.questions`. Leave the column unset on the ActiveModel.
pub async fn persist_job_questions<C: ConnectionTrait>(
    db: &C,
    id: i64,
    questions: &JobPostingQuestions,
) -> Result<(), String> {
    let backend = db.get_database_backend();
    let (sql, values) = if questions.is_empty() {
        (
            "UPDATE hr_job_forms SET questions = '{}'::hr_job_posting_question[] WHERE id = $1"
                .to_string(),
            vec![Value::BigInt(Some(id))],
        )
    } else {
        let mut sql = String::from("UPDATE hr_job_forms SET questions = ARRAY[");
        let mut values = Vec::new();
        for (i, q) in questions.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            let base = i * 5;
            sql.push_str(&format!(
                "ROW(${}::uuid, ${}, ${}::jsonb, ${}, ${})::hr_job_posting_question",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5
            ));
            values.push(Value::Uuid(Some(q.job_posting_question_id)));
            values.push(Value::String(Some(q.display_text.clone())));
            values.push(json_value(&q.question_type.0));
            values.push(Value::Bool(Some(q.required)));
            values.push(Value::String(Some(q.description.clone())));
        }
        sql.push_str(&format!(
            "]::hr_job_posting_question[] WHERE id = ${}",
            values.len() + 1
        ));
        values.push(Value::BigInt(Some(id)));
        (sql, values)
    };
    db.execute_raw(Statement::from_sql_and_values(backend, sql, values))
        .await
        .map_err(|e: DbErr| e.to_string())?;
    Ok(())
}

/// Replace `hr_applicants.answers`. Leave the column unset on the ActiveModel.
pub async fn persist_applicant_answers<C: ConnectionTrait>(
    db: &C,
    id: i64,
    answers: &JobPostingAnswers,
) -> Result<(), String> {
    let backend = db.get_database_backend();
    let (sql, values) = if answers.is_empty() {
        (
            "UPDATE hr_applicants SET answers = '{}'::hr_job_posting_answer[] WHERE id = $1"
                .to_string(),
            vec![Value::BigInt(Some(id))],
        )
    } else {
        let mut sql = String::from("UPDATE hr_applicants SET answers = ARRAY[");
        let mut values = Vec::new();
        for (i, row) in answers.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            let base = i * 6;
            let q = &row.question;
            sql.push_str(&format!(
                "ROW(ROW(${}::uuid, ${}, ${}::jsonb, ${}, ${})::hr_job_posting_question, ${}::jsonb)::hr_job_posting_answer",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
                base + 6
            ));
            values.push(Value::Uuid(Some(q.job_posting_question_id)));
            values.push(Value::String(Some(q.display_text.clone())));
            values.push(json_value(&q.question_type.0));
            values.push(Value::Bool(Some(q.required)));
            values.push(Value::String(Some(q.description.clone())));
            values.push(json_value(&row.answer.0));
        }
        sql.push_str(&format!(
            "]::hr_job_posting_answer[] WHERE id = ${}",
            values.len() + 1
        ));
        values.push(Value::BigInt(Some(id)));
        (sql, values)
    };
    db.execute_raw(Statement::from_sql_and_values(backend, sql, values))
        .await
        .map_err(|e: DbErr| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lariv_plugin_forms::types::{McqGridAnswer, RatingMarker};

    fn q(id: &str, text: &str, qt: FormQuestionType, required: bool) -> JobPostingQuestion {
        JobPostingQuestion {
            job_posting_question_id: Uuid::parse_str(id).expect("uuid"),
            display_text: text.into(),
            question_type: Json(qt),
            required,
            description: String::new(),
        }
    }

    #[test]
    fn blank_description_round_trips_as_empty_string() {
        let raw = r#"[{"form_question_id":"11111111-1111-1111-1111-111111111111","display_text":"Name","question_type":"ShortText","required":true,"description":null}]"#;
        let parsed = parse_questions_json(raw).expect("parse");
        assert_eq!(parsed[0].description, "");
        let json = questions_editor_json(&parsed);
        assert!(json.contains("null") || !json.contains("\"description\":\"\""));
    }

    #[test]
    fn duplicate_question_id_is_rejected() {
        let raw = r#"[{"form_question_id":"11111111-1111-1111-1111-111111111111","display_text":"A","question_type":"ShortText","required":false},{"form_question_id":"11111111-1111-1111-1111-111111111111","display_text":"B","question_type":"ShortText","required":false}]"#;
        let err = parse_questions_json(raw).expect_err("dup");
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn answer_snapshots_the_question_and_rejects_unknown_ids() {
        let questions = JobPostingQuestions(vec![q(
            "11111111-1111-1111-1111-111111111111",
            "Score",
            FormQuestionType::Number(NumberRange {
                start: Some(0.0),
                end: Some(10.0),
            }),
            true,
        )]);
        let answers = parse_answers_json(
            r#"{"11111111-1111-1111-1111-111111111111":{"Number":4}}"#,
            &questions,
        )
        .expect("answers");
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].question.display_text, "Score");
        assert!(
            parse_answers_json(
                r#"{"22222222-2222-2222-2222-222222222222":{"Number":1}}"#,
                &questions,
            )
            .is_err()
        );
        assert!(
            parse_answers_json(
                r#"{"11111111-1111-1111-1111-111111111111":{"Number":11}}"#,
                &questions,
            )
            .is_err()
        );
    }

    #[test]
    fn choice_must_be_declared() {
        let questions = JobPostingQuestions(vec![q(
            "11111111-1111-1111-1111-111111111111",
            "Color",
            FormQuestionType::MCQText(vec!["Red".into(), "Blue".into()]),
            true,
        )]);
        assert!(
            parse_answers_json(
                r#"{"11111111-1111-1111-1111-111111111111":{"MCQText":"Green"}}"#,
                &questions,
            )
            .is_err()
        );
    }

    #[test]
    fn render_uses_embedded_question() {
        let answers = JobPostingAnswers(vec![JobPostingAnswer {
            question: q(
                "11111111-1111-1111-1111-111111111111",
                "Stars",
                FormQuestionType::Rating(RatingSpec {
                    range: 5,
                    marker: RatingMarker::Star,
                }),
                false,
            ),
            answer: Json(FormAnswer::Rating(4)),
        }]);
        let rendered = render_answers(&answers);
        assert_eq!(rendered[0].display_text, "Stars");
        assert_eq!(rendered[0].answer, "4");
        let _ = McqGridAnswer::default();
    }
}
