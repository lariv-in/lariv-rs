//! Phone numbers are required strings. SQL NULL decodes as empty.

use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, ValueType, ValueTypeErr};
use sea_orm::{ColIdx, TryGetable};
use serde::{Deserialize, Serialize};

/// Non-null phone stored as text. Database NULL is read as an empty string.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phone(String);

impl Phone {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for Phone {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Phone {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<Phone> for String {
    fn from(value: Phone) -> Self {
        value.0
    }
}

impl std::ops::Deref for Phone {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Phone {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Phone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Phone> for Value {
    fn from(source: Phone) -> Self {
        source.0.into()
    }
}

impl TryGetable for Phone {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        match Option::<String>::try_get_by(res, idx) {
            Ok(Some(value)) => Ok(Self(value)),
            Ok(None) => Ok(Self::default()),
            Err(TryGetError::Null(_)) => Ok(Self::default()),
            Err(err) => Err(err),
        }
    }
}

impl ValueType for Phone {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <String as ValueType>::try_from(v).map(Self)
    }

    fn type_name() -> String {
        "Phone".to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::String
    }

    fn column_type() -> ColumnType {
        ColumnType::Text
    }
}

impl Nullable for Phone {
    fn null() -> Value {
        <String as Nullable>::null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_null_is_empty() {
        assert_eq!(Phone::default().as_str(), "");
        assert_eq!(String::from(Phone::from("123")), "123");
    }
}
