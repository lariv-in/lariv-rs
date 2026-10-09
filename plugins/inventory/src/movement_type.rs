//! In / out direction stored as a short string on stock movements.

use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const MOVEMENT_IN: &str = "in";
pub const MOVEMENT_OUT: &str = "out";

/// Direction of every line on a stock movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum MovementType {
    #[sea_orm(string_value = "in")]
    In,
    #[sea_orm(string_value = "out")]
    Out,
}

impl MovementType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::In => MOVEMENT_IN,
            Self::Out => MOVEMENT_OUT,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::In => "In",
            Self::Out => "Out",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            MOVEMENT_IN => Some(Self::In),
            MOVEMENT_OUT => Some(Self::Out),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[(MOVEMENT_IN, "In"), (MOVEMENT_OUT, "Out")]
    }

    pub fn is_out(self) -> bool {
        matches!(self, Self::Out)
    }
}

impl fmt::Display for MovementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for MovementType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid movement type: {s:?}"))
    }
}
