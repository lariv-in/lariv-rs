//! Text columns that decode SQL NULL as an empty string.

use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, ValueType, ValueTypeErr};
use sea_orm::{ColIdx, TryGetable};
use serde::{Deserialize, Serialize};

/// Non-null text stored as text. Database NULL is read as empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullText(String);

impl NullText {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for NullText {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for NullText {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<NullText> for String {
    fn from(value: NullText) -> Self {
        value.0
    }
}

impl std::ops::Deref for NullText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for NullText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NullText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<str> for NullText {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for NullText {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for NullText {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl From<NullText> for Value {
    fn from(source: NullText) -> Self {
        source.0.into()
    }
}

impl TryGetable for NullText {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        match Option::<String>::try_get_by(res, idx) {
            Ok(Some(value)) => Ok(Self(value)),
            Ok(None) => Ok(Self::default()),
            Err(TryGetError::Null(_)) => Ok(Self::default()),
            Err(err) => Err(err),
        }
    }
}

impl ValueType for NullText {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <String as ValueType>::try_from(v).map(Self)
    }

    fn type_name() -> String {
        "NullText".to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::String
    }

    fn column_type() -> ColumnType {
        ColumnType::Text
    }
}

impl Nullable for NullText {
    fn null() -> Value {
        <String as Nullable>::null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_null_is_empty() {
        assert_eq!(NullText::default().as_str(), "");
        assert_eq!(String::from(NullText::from("a@b.c")), "a@b.c");
        assert_eq!(NullText::from("x"), "x");
    }
}
