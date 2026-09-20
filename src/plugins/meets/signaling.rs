//! JSON signaling protocol for the meets SFU (not HTMX).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Join {
        joined_user_id: i64,
        #[serde(default)]
        video_codecs: Vec<String>,
    },
    Answer {
        sdp: String,
    },
    Ice {
        candidate: String,
        #[serde(default)]
        sdp_mid: Option<String>,
        #[serde(default)]
        sdp_mline_index: Option<u16>,
    },
    Leave,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Offer {
        sdp: String,
    },
    Ice {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },
    Codec {
        video: String,
        audio: String,
    },
    ParticipantJoined {
        joined_user_id: i64,
    },
    ParticipantLeft {
        joined_user_id: i64,
    },
    TracksChanged {
        joined_user_id: i64,
        kind: String,
    },
    Error {
        message: String,
    },
}

impl ServerMsg {
    pub fn to_text(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| r#"{"type":"error","message":"encode failed"}"#.into())
    }
}
