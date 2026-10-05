use std::fmt;
use std::str::FromStr;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const LEAD_SOURCE_WEB_FORM: &str = "web_form";
pub const LEAD_SOURCE_COLD_OUTREACH: &str = "cold_outreach";
pub const LEAD_SOURCE_SCRAPE_LIST: &str = "scrape_list";
pub const LEAD_SOURCE_REFERRAL: &str = "referral";
pub const LEAD_SOURCE_OTHER: &str = "other";

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum LeadSource {
    #[default]
    #[sea_orm(string_value = "web_form")]
    WebForm,
    #[sea_orm(string_value = "cold_outreach")]
    ColdOutreach,
    #[sea_orm(string_value = "scrape_list")]
    ScrapeList,
    #[sea_orm(string_value = "referral")]
    Referral,
    #[sea_orm(string_value = "other")]
    Other,
}

impl LeadSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WebForm => LEAD_SOURCE_WEB_FORM,
            Self::ColdOutreach => LEAD_SOURCE_COLD_OUTREACH,
            Self::ScrapeList => LEAD_SOURCE_SCRAPE_LIST,
            Self::Referral => LEAD_SOURCE_REFERRAL,
            Self::Other => LEAD_SOURCE_OTHER,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::WebForm => "Web form",
            Self::ColdOutreach => "Cold outreach",
            Self::ScrapeList => "Scrape list",
            Self::Referral => "Referral",
            Self::Other => "Other",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            LEAD_SOURCE_WEB_FORM => Some(Self::WebForm),
            LEAD_SOURCE_COLD_OUTREACH => Some(Self::ColdOutreach),
            LEAD_SOURCE_SCRAPE_LIST => Some(Self::ScrapeList),
            LEAD_SOURCE_REFERRAL => Some(Self::Referral),
            LEAD_SOURCE_OTHER => Some(Self::Other),
            _ => None,
        }
    }

    pub fn choices() -> &'static [(&'static str, &'static str)] {
        &[
            (LEAD_SOURCE_WEB_FORM, "Web form"),
            (LEAD_SOURCE_COLD_OUTREACH, "Cold outreach"),
            (LEAD_SOURCE_SCRAPE_LIST, "Scrape list"),
            (LEAD_SOURCE_REFERRAL, "Referral"),
            (LEAD_SOURCE_OTHER, "Other"),
        ]
    }
}

impl fmt::Display for LeadSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for LeadSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid LeadSource: {s:?}"))
    }
}
