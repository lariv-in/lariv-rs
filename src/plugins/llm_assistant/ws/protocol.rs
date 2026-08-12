//! HTMX 4 `hx-ws` envelope + chat user message.

use serde::Deserialize;

use crate::html_form::{json_flex_i64, json_flex_vec_i64};

/// HTMX 4 outgoing WebSocket JSON: `{ headers, body }`.
#[derive(Debug, Deserialize)]
pub struct HtmxWsEnvelope {
    #[serde(default)]
    pub body: UserMessageBody,
}

#[derive(Debug, Default, Deserialize)]
pub struct UserMessageBody {
    #[serde(default, deserialize_with = "json_flex_i64")]
    pub session_id: i64,
    #[serde(default)]
    pub message: String,
    #[serde(default, alias = "Files", alias = "files", deserialize_with = "json_flex_vec_i64")]
    pub files: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WsIncoming {
    Envelope(HtmxWsEnvelope),
    Flat(UserMessageBody),
}

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub session_id: i64,
    pub message: String,
    pub files: Vec<i64>,
}

impl UserMessage {
    pub fn from_envelope(raw: &str) -> Result<Self, String> {
        let incoming: WsIncoming =
            serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
        let body = match incoming {
            WsIncoming::Envelope(env) => env.body,
            WsIncoming::Flat(body) => body,
        };
        Ok(Self {
            session_id: body.session_id,
            message: body.message.trim().to_string(),
            files: body.files,
        })
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
