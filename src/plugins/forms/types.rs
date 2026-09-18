//! Typed question and answer payloads stored in form JSON columns.

use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};

use chrono::{NaiveDate, NaiveTime};
use sea_orm::TryGetableFromJson;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, ValueType, ValueTypeErr};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable id for a question, generated on create so reordering does not change it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FormQuestionId(pub Uuid);

impl FormQuestionId {
    /// Allocate a new random question id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for FormQuestionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Uuid> for FormQuestionId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<FormQuestionId> for Uuid {
    fn from(id: FormQuestionId) -> Self {
        id.0
    }
}

/// One question on a form. `form_question_id` is stable across list reordering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormQuestion {
    pub form_question_id: FormQuestionId,
    pub display_text: String,
    pub question_type: FormQuestionType,
    pub required: bool,
    pub description: Option<String>,
}

impl FormQuestion {
    /// Build a question with a fresh [`FormQuestionId`].
    pub fn new(
        display_text: impl Into<String>,
        question_type: FormQuestionType,
        required: bool,
    ) -> Self {
        Self {
            form_question_id: FormQuestionId::new(),
            display_text: display_text.into(),
            question_type,
            required,
            description: None,
        }
    }
}

/// Inclusive numeric bounds for a [`FormQuestionType::Number`] question.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumberRange {
    pub start: Option<f64>,
    pub end: Option<f64>,
}

/// Inclusive integer scale with optional endpoint labels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinearScaleSpec {
    pub start: i32,
    pub end: i32,
    pub start_label: Option<String>,
    pub end_label: Option<String>,
}

/// Rating widget: max value and marker icon.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatingSpec {
    pub range: u32,
    pub marker: RatingMarker,
}

/// Icon used by [`FormQuestionType::Rating`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RatingMarker {
    Star,
    Heart,
    Like,
}

/// Row and column labels for grid questions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridLabels {
    pub rows: Vec<String>,
    pub cols: Vec<String>,
}

/// Single selected cell for an MCQ grid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct McqGridAnswer {
    pub row: String,
    pub col: String,
}

/// Selected row and column labels for a tick-box grid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickBoxGridAnswer {
    pub rows: Vec<String>,
    pub cols: Vec<String>,
}

/// Question widget kind and type-specific options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FormQuestionType {
    ShortText,
    LongText,
    Number(NumberRange),
    MCQText(Vec<String>),
    MCQWithCustom(Vec<String>),
    Checkboxes(Vec<String>),
    Dropdown(Vec<String>),
    LinearScale(LinearScaleSpec),
    Rating(RatingSpec),
    MCQGrid(GridLabels),
    TickBoxGrid(GridLabels),
    Date,
    Time,
}

/// Submitted value for one question. Variant should match the question type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FormAnswer {
    ShortText(String),
    LongText(String),
    Number(f64),
    MCQText(String),
    MCQWithCustom(String),
    Checkboxes(Vec<String>),
    Dropdown(String),
    LinearScale(i32),
    Rating(u32),
    MCQGrid(McqGridAnswer),
    TickBoxGrid(TickBoxGridAnswer),
    Date(NaiveDate),
    Time(NaiveTime),
}

/// JSON column wrapper: ordered list of questions on a form.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FormQuestions(pub Vec<FormQuestion>);

impl Deref for FormQuestions {
    type Target = Vec<FormQuestion>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FormQuestions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<FormQuestion>> for FormQuestions {
    fn from(questions: Vec<FormQuestion>) -> Self {
        Self(questions)
    }
}

/// JSON column wrapper: answers keyed by stable question id.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FormAnswers(pub HashMap<FormQuestionId, FormAnswer>);

impl Deref for FormAnswers {
    type Target = HashMap<FormQuestionId, FormAnswer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FormAnswers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HashMap<FormQuestionId, FormAnswer>> for FormAnswers {
    fn from(answers: HashMap<FormQuestionId, FormAnswer>) -> Self {
        Self(answers)
    }
}

macro_rules! impl_json_column {
    ($ty:ty, $name:literal) => {
        impl TryGetableFromJson for $ty {}

        impl From<$ty> for Value {
            fn from(source: $ty) -> Self {
                Value::Json(serde_json::to_value(source).ok().map(Box::new))
            }
        }

        impl ValueType for $ty {
            fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
                match v {
                    Value::Json(Some(json)) => {
                        serde_json::from_value(*json).map_err(|_| ValueTypeErr)
                    }
                    _ => Err(ValueTypeErr),
                }
            }

            fn type_name() -> String {
                $name.to_owned()
            }

            fn array_type() -> ArrayType {
                ArrayType::Json
            }

            fn column_type() -> ColumnType {
                ColumnType::JsonBinary
            }
        }

        impl Nullable for $ty {
            fn null() -> Value {
                Value::Json(None)
            }
        }
    };
}

impl_json_column!(FormQuestions, "FormQuestions");
impl_json_column!(FormAnswers, "FormAnswers");

#[cfg(test)]
mod tests {
    use super::*;

    fn qid(s: &str) -> FormQuestionId {
        FormQuestionId(Uuid::parse_str(s).expect("uuid"))
    }

    fn sample_questions() -> FormQuestions {
        FormQuestions(vec![
            FormQuestion {
                form_question_id: qid("11111111-1111-1111-1111-111111111111"),
                display_text: "Name".into(),
                question_type: FormQuestionType::ShortText,
                required: true,
                description: Some("Your full name".into()),
            },
            FormQuestion {
                form_question_id: qid("22222222-2222-2222-2222-222222222222"),
                display_text: "Score".into(),
                question_type: FormQuestionType::Number(NumberRange {
                    start: Some(0.0),
                    end: Some(10.0),
                }),
                required: false,
                description: None,
            },
            FormQuestion {
                form_question_id: qid("33333333-3333-3333-3333-333333333333"),
                display_text: "Color".into(),
                question_type: FormQuestionType::MCQText(vec!["Red".into(), "Blue".into()]),
                required: true,
                description: None,
            },
            FormQuestion {
                form_question_id: qid("44444444-4444-4444-4444-444444444444"),
                display_text: "When".into(),
                question_type: FormQuestionType::Date,
                required: false,
                description: None,
            },
            FormQuestion {
                form_question_id: qid("55555555-5555-5555-5555-555555555555"),
                display_text: "Grid".into(),
                question_type: FormQuestionType::MCQGrid(GridLabels {
                    rows: vec!["R1".into()],
                    cols: vec!["C1".into(), "C2".into()],
                }),
                required: false,
                description: None,
            },
            FormQuestion {
                form_question_id: qid("66666666-6666-6666-6666-666666666666"),
                display_text: "Clock".into(),
                question_type: FormQuestionType::Time,
                required: false,
                description: None,
            },
        ])
    }

    fn sample_answers() -> FormAnswers {
        let mut answers = HashMap::new();
        answers.insert(
            qid("11111111-1111-1111-1111-111111111111"),
            FormAnswer::ShortText("Ada".into()),
        );
        answers.insert(
            qid("22222222-2222-2222-2222-222222222222"),
            FormAnswer::Number(7.5),
        );
        answers.insert(
            qid("33333333-3333-3333-3333-333333333333"),
            FormAnswer::MCQText("Blue".into()),
        );
        answers.insert(
            qid("44444444-4444-4444-4444-444444444444"),
            FormAnswer::Date(NaiveDate::from_ymd_opt(2026, 9, 18).expect("date")),
        );
        answers.insert(
            qid("55555555-5555-5555-5555-555555555555"),
            FormAnswer::MCQGrid(McqGridAnswer {
                row: "R1".into(),
                col: "C2".into(),
            }),
        );
        answers.insert(
            qid("66666666-6666-6666-6666-666666666666"),
            FormAnswer::Time(NaiveTime::from_hms_opt(14, 30, 0).expect("time")),
        );
        FormAnswers(answers)
    }

    #[test]
    fn questions_json_round_trip() {
        let original = sample_questions();
        let json = serde_json::to_value(&original).expect("serialize questions");
        let restored: FormQuestions = serde_json::from_value(json).expect("deserialize questions");
        assert_eq!(restored, original);
    }

    #[test]
    fn answers_json_round_trip_keeps_uuid_keys() {
        let original = sample_answers();
        let json = serde_json::to_value(&original).expect("serialize answers");
        let obj = json.as_object().expect("answers object");
        assert!(obj.contains_key("11111111-1111-1111-1111-111111111111"));
        let restored: FormAnswers = serde_json::from_value(json).expect("deserialize answers");
        assert_eq!(restored, original);
    }

    #[test]
    fn new_question_allocates_id() {
        let q = FormQuestion::new("Hello", FormQuestionType::LongText, false);
        assert_ne!(q.form_question_id.0, Uuid::nil());
        assert_eq!(q.display_text, "Hello");
        assert!(!q.required);
        assert!(q.description.is_none());
    }
}
