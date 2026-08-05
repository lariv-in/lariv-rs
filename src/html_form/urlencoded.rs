//! `application/x-www-form-urlencoded` parsing that preserves duplicate field names.
//!
//! Axum's [`axum::Form`] uses `serde_urlencoded`, which rejects duplicate keys. HTML
//! many-to-many pickers emit one hidden input per selected id (`TaxIds=9&TaxIds=8`).
//! Use [`deserialize_urlencoded`] or the [`super::HtmlFormBody`] extractor instead.

use std::collections::HashMap;

use serde::de::DeserializeOwned;

use super::FormError;
use super::multipart::deserialize_text_map;

/// Parse a urlencoded body into a map of field names to all submitted values.
pub fn parse_urlencoded_form(body: &[u8]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (key, value) in form_urlencoded::parse(body) {
        map.entry(key.into_owned())
            .or_default()
            .push(value.into_owned());
    }
    map
}

/// Deserialize an urlencoded form body into `T`.
///
/// Duplicate keys become JSON arrays so [`super::form_vec_i64`] / [`super::form_vec_string`]
/// on `#[html_form]` fields work correctly.
pub fn deserialize_urlencoded<T: DeserializeOwned>(body: &[u8]) -> Result<T, FormError> {
    let text = parse_urlencoded_form(body);
    deserialize_text_map(&text)
}

#[cfg(test)]
mod tests {
    use super::deserialize_urlencoded;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct ProductLikeForm {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "TaxIds", default, deserialize_with = "crate::html_form::form_vec_i64")]
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
}
