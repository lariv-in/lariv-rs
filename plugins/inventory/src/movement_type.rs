//! Kind of stock movement: in, out, or process.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const MOVEMENT_IN: &str = "in";
pub const MOVEMENT_OUT: &str = "out";
pub const MOVEMENT_PROCESS: &str = "process";

/// Which stock-movement table a document lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MovementKind {
    In,
    Out,
    Process,
}

impl MovementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::In => MOVEMENT_IN,
            Self::Out => MOVEMENT_OUT,
            Self::Process => MOVEMENT_PROCESS,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::In => "In",
            Self::Out => "Out",
            Self::Process => "Process",
        }
    }

    /// `{{TYPE}}` token in the movement number format.
    pub fn type_token(self) -> &'static str {
        match self {
            Self::In => "IN",
            Self::Out => "OUT",
            Self::Process => "PROCESS",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            MOVEMENT_IN => Some(Self::In),
            MOVEMENT_OUT => Some(Self::Out),
            MOVEMENT_PROCESS => Some(Self::Process),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (MOVEMENT_IN, "In"),
            (MOVEMENT_OUT, "Out"),
            (MOVEMENT_PROCESS, "Process"),
        ]
    }

    pub fn has_party(self) -> bool {
        matches!(self, Self::In | Self::Out)
    }

    pub fn has_pdf(self) -> bool {
        matches!(self, Self::In | Self::Out)
    }
}

impl fmt::Display for MovementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for MovementKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid movement type: {s:?}"))
    }
}
