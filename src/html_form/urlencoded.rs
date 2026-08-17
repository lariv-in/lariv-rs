//! `application/x-www-form-urlencoded` parsing that preserves duplicate field names.
//!
//! Axum's [`axum::Form`] uses `serde_urlencoded`, which rejects duplicate keys. HTML
//! many-to-many pickers emit one hidden input per selected id (`TaxIds=9&TaxIds=8`).
//! Use [`UrlencodedFields`] or the [`super::HtmlFormBody`] extractor instead.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde_json::Map;

use super::FormError;

/// Parsed urlencoded body preserving duplicate field names.
#[derive(Debug, Clone, Default)]
pub struct UrlencodedFields {
    pairs: Vec<(String, String)>,
}

impl UrlencodedFields {
    /// Parse raw urlencoded bytes into ordered field pairs.
    pub fn parse(body: &[u8]) -> Result<Self, FormError> {
        let mut pairs = Vec::new();
        for (key, value) in form_urlencoded::parse(body) {
            pairs.push((key.into_owned(), value.into_owned()));
        }
        Ok(Self { pairs })
    }

    /// Append a text field (preserves duplicate keys).
    pub fn push(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.pairs.push((key.into(), value.into()));
    }

    /// First value for `key`, if any.
    pub fn get_first(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Deserialize into `T`, folding duplicate keys into JSON arrays.
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, FormError> {
        let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
        for (key, value) in &self.pairs {
            grouped.entry(key.clone()).or_default().push(value.clone());
        }
        let map: Map<String, serde_json::Value> = grouped
            .into_iter()
            .map(|(k, v)| {
                let value = if v.is_empty() {
                    serde_json::Value::String(String::new())
                } else if v.len() == 1 {
                    serde_json::Value::String(v[0].clone())
                } else {
                    serde_json::Value::Array(v.into_iter().map(serde_json::Value::String).collect())
                };
                (k, value)
            })
            .collect();
        serde_json::from_value(serde_json::Value::Object(map))
            .map_err(|e| FormError::Deserialize(e.to_string()))
    }
}

/// Deserialize an urlencoded form body into `T`.
pub fn deserialize_urlencoded<T: DeserializeOwned>(body: &[u8]) -> Result<T, FormError> {
    UrlencodedFields::parse(body)?.deserialize()
}

#[cfg(test)]
mod tests {
    use super::deserialize_urlencoded;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct ProductLikeForm {
        #[serde(rename = "Name")]
        name: String,
        #[serde(
            rename = "TaxIds",
            default,
            deserialize_with = "crate::html_form::form_vec_i64"
        )]
        tax_ids: Vec<i64>,
    }

    #[test]
    fn duplicate_many_to_many_keys_deserialize() {
        let body = b"Name=Widget&TaxIds=9&TaxIds=8";
        let form: ProductLikeForm = deserialize_urlencoded(body).expect("duplicate TaxIds");
        assert_eq!(form.name, "Widget");
        assert_eq!(form.tax_ids, vec![9, 8]);
    }

    #[test]
    fn single_many_to_many_key_deserializes() {
        let body = b"Name=Widget&TaxIds=9";
        let form: ProductLikeForm = deserialize_urlencoded(body).expect("single TaxIds");
        assert_eq!(form.tax_ids, vec![9]);
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ModelsForm {
        #[serde(default, deserialize_with = "crate::html_form::form_vec_string")]
        models: Vec<String>,
    }

    #[test]
    fn duplicate_models_keys_deserialize() {
        let body = b"models=roles&models=users&models=otp_preferences";
        let form: ModelsForm = deserialize_urlencoded(body).expect("duplicate models");
        assert_eq!(
            form.models,
            vec![
                "roles".to_string(),
                "users".to_string(),
                "otp_preferences".to_string()
            ]
        );
    }
}
