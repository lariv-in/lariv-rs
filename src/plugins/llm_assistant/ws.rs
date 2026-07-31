//! WebSocket protocol + OOB HTML for assistant chat (HTMX 4 hx-ws).

pub mod html;
pub mod protocol;

pub use html::*;
pub use protocol::{HtmxWsEnvelope, UserMessage};
