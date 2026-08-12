use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const DEAL_STAGE_PROSPECTING: &str = "prospecting";
pub const DEAL_STAGE_QUALIFICATION: &str = "qualification";
pub const DEAL_STAGE_PROPOSAL: &str = "proposal";
pub const DEAL_STAGE_NEGOTIATION: &str = "negotiation";
pub const DEAL_STAGE_CLOSED_WON: &str = "closed_won";
pub const DEAL_STAGE_CLOSED_LOST: &str = "closed_lost";

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum DealStage {
    #[default]
    #[sea_orm(string_value = "prospecting")]
    Prospecting,
    #[sea_orm(string_value = "qualification")]
    Qualification,
    #[sea_orm(string_value = "proposal")]
    Proposal,
    #[sea_orm(string_value = "negotiation")]
    Negotiation,
    #[sea_orm(string_value = "closed_won")]
    ClosedWon,
    #[sea_orm(string_value = "closed_lost")]
    ClosedLost,
}

impl DealStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prospecting => DEAL_STAGE_PROSPECTING,
            Self::Qualification => DEAL_STAGE_QUALIFICATION,
            Self::Proposal => DEAL_STAGE_PROPOSAL,
            Self::Negotiation => DEAL_STAGE_NEGOTIATION,
            Self::ClosedWon => DEAL_STAGE_CLOSED_WON,
            Self::ClosedLost => DEAL_STAGE_CLOSED_LOST,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Prospecting => "Prospecting",
            Self::Qualification => "Qualification",
            Self::Proposal => "Proposal",
            Self::Negotiation => "Negotiation",
            Self::ClosedWon => "Closed Won",
            Self::ClosedLost => "Closed Lost",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            DEAL_STAGE_PROSPECTING => Some(Self::Prospecting),
            DEAL_STAGE_QUALIFICATION => Some(Self::Qualification),
            DEAL_STAGE_PROPOSAL => Some(Self::Proposal),
            DEAL_STAGE_NEGOTIATION => Some(Self::Negotiation),
            DEAL_STAGE_CLOSED_WON => Some(Self::ClosedWon),
            DEAL_STAGE_CLOSED_LOST => Some(Self::ClosedLost),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (DEAL_STAGE_PROSPECTING, "Prospecting"),
            (DEAL_STAGE_QUALIFICATION, "Qualification"),
            (DEAL_STAGE_PROPOSAL, "Proposal"),
            (DEAL_STAGE_NEGOTIATION, "Negotiation"),
            (DEAL_STAGE_CLOSED_WON, "Closed Won"),
            (DEAL_STAGE_CLOSED_LOST, "Closed Lost"),
        ]
    }
}

impl fmt::Display for DealStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for DealStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid DealStage: {s:?}"))
    }
}
