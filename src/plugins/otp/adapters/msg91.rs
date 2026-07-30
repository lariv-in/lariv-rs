use serde_json::{Map, Value};
use thiserror::Error;

pub const MSG91_BASE_URL: &str = "https://control.msg91.com/api/v5";

pub type FlowRecipient = Map<String, Value>;

#[derive(Debug, Error)]
pub enum Msg91Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("MSG91 API error: status {status}, response: {body}")]
    Api { status: u16, body: String },
    #[error("failed to parse response JSON: {0}")]
    Json(String),
}

pub struct Msg91Client {
    auth_key: String,
    http: reqwest::Client,
}

impl Msg91Client {
    pub fn new(auth_key: String) -> Self {
        Self {
            auth_key,
            http: reqwest::Client::new(),
        }
    }

    /// Send an SMS using the MSG91 Flow API.
    pub async fn send_sms_flow(
        &self,
        template_id: &str,
        recipients: Vec<FlowRecipient>,
        real_time_response: bool,
    ) -> Result<Map<String, Value>, Msg91Error> {
        let url = format!("{MSG91_BASE_URL}/flow");
        let mut payload = serde_json::json!({
            "template_id": template_id,
            "short_url": "1",
            "recipients": recipients,
        });
        if real_time_response {
            payload["realTimeResponse"] = Value::String("1".into());
        }

        let resp = self
            .http
            .post(&url)
            .header("authkey", &self.auth_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Msg91Error::Api {
                status: status.as_u16(),
                body,
            });
        }

        serde_json::from_str(&body).map_err(|e| Msg91Error::Json(format!("{e} (body: {body})")))
    }
}
