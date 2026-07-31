//! HTMX 4 `hx-ws` envelope + chat user message.

use serde::Deserialize;
use serde_json::Value;

/// HTMX 4 outgoing WebSocket JSON: `{ headers, body }`.
#[derive(Debug, Deserialize)]
pub struct HtmxWsEnvelope {
    #[serde(default)]
    pub body: Value,
}

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub session_id: i64,
    pub message: String,
    pub files: Vec<i64>,
}

impl UserMessage {
    pub fn from_envelope(raw: &str) -> Result<Self, String> {
        let v: Value =
            serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
        let body = if let Some(body) = v.get("body") {
            body.clone()
        } else {
            // Flat Go-style payload fallback
            v
        };
        let session_id = parse_session_id(body.get("session_id"));
        let message = body
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let files = parse_files(body.get("Files").or_else(|| body.get("files")));
        Ok(Self {
            session_id,
            message,
            files,
        })
    }
}

fn parse_session_id(v: Option<&Value>) -> i64 {
    match v {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

fn parse_files(v: Option<&Value>) -> Vec<i64> {
    match v {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() {
                Vec::new()
            } else {
                s.parse().ok().into_iter().collect()
            }
        }
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|item| match item {
                Value::Number(n) => n.as_i64(),
                Value::String(s) => s.trim().parse().ok(),
                _ => None,
            })
            .collect(),
        Some(Value::Number(n)) => n.as_i64().into_iter().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::UserMessage;

    #[test]
    fn parses_htmx4_envelope_body() {
        let raw = r#"{"headers":{"HX-Request":"true"},"body":{"session_id":12,"message":"hi","Files":["3","4"]}}"#;
        let msg = UserMessage::from_envelope(raw).unwrap();
        assert_eq!(msg.session_id, 12);
        assert_eq!(msg.message, "hi");
        assert_eq!(msg.files, vec![3, 4]);
    }

    #[test]
    fn parses_string_session_and_single_file() {
        let raw = r#"{"body":{"session_id":"0","message":" x ","Files":"9"}}"#;
        let msg = UserMessage::from_envelope(raw).unwrap();
        assert_eq!(msg.session_id, 0);
        assert_eq!(msg.message, "x");
        assert_eq!(msg.files, vec![9]);
    }
}
