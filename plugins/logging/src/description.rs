//! Turn a stored audit description into the two JSON documents a diff needs.

use serde_json::{Map, Value};

/// Left and right JSON text for one log row.
///
/// Create diffs against `null`. Delete diffs from the stored row to `null`.
/// Update rows stored as `{"before","after"}` use those full documents. Older
/// update rows stored only changed fields as `{"field":{"from","to"}}`.
pub fn log_description_sides(kind: &str, description: &Value) -> (String, String) {
    match kind {
        "Create" => ("null".to_string(), embed(description)),
        "Delete" => (embed(description), "null".to_string()),
        "Update" => update_sides(description),
        _ => ("null".to_string(), embed(description)),
    }
}

fn embed(value: &Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "null".to_string())
        .replace('<', "\\u003c")
}

fn update_sides(description: &Value) -> (String, String) {
    let Some(fields) = description.as_object() else {
        return ("null".to_string(), embed(description));
    };
    if is_before_after(fields) {
        let before = fields.get("before").cloned().unwrap_or(Value::Null);
        let after = fields.get("after").cloned().unwrap_or(Value::Null);
        return (embed(&before), embed(&after));
    }
    let mut left = Map::new();
    let mut right = Map::new();
    for (key, child) in fields {
        let Some(pair) = child.as_object() else {
            return ("null".to_string(), embed(description));
        };
        if !is_from_to(pair) {
            return ("null".to_string(), embed(description));
        }
        left.insert(
            key.clone(),
            pair.get("from").cloned().unwrap_or(Value::Null),
        );
        right.insert(key.clone(), pair.get("to").cloned().unwrap_or(Value::Null));
    }
    (embed(&Value::Object(left)), embed(&Value::Object(right)))
}

fn is_before_after(fields: &Map<String, Value>) -> bool {
    !fields.is_empty()
        && fields.keys().all(|key| key == "before" || key == "after")
        && fields.values().any(|value| !is_from_to_value(value))
}

fn is_from_to_value(value: &Value) -> bool {
    value.as_object().is_some_and(is_from_to)
}

fn is_from_to(fields: &Map<String, Value>) -> bool {
    !fields.is_empty() && fields.keys().all(|key| key == "from" || key == "to")
}

#[cfg(test)]
mod tests {
    use super::log_description_sides;
    use serde_json::json;

    #[test]
    fn create_and_delete_use_null_on_the_empty_side() {
        let row = json!({"name": "a"});
        let (left, right) = log_description_sides("Create", &row);
        assert_eq!(left, "null");
        assert_eq!(right, r#"{"name":"a"}"#);
        let (left, right) = log_description_sides("Delete", &row);
        assert_eq!(left, r#"{"name":"a"}"#);
        assert_eq!(right, "null");
    }

    #[test]
    fn update_uses_full_before_and_after() {
        let description = json!({"before": {"name": "a", "n": 1}, "after": {"name": "b", "n": 1}});
        let (left, right) = log_description_sides("Update", &description);
        let left: serde_json::Value = serde_json::from_str(&left).expect("left json");
        let right: serde_json::Value = serde_json::from_str(&right).expect("right json");
        assert_eq!(left, json!({"name": "a", "n": 1}));
        assert_eq!(right, json!({"name": "b", "n": 1}));
    }

    #[test]
    fn legacy_update_rebuilds_changed_fields_only() {
        let description = json!({"name": {"from": "a", "to": "b"}});
        let (left, right) = log_description_sides("Update", &description);
        assert_eq!(left, r#"{"name":"a"}"#);
        assert_eq!(right, r#"{"name":"b"}"#);
    }
}
