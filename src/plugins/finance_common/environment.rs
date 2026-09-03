//! Lariv `environment` cookie — JSON object of string keys, shared across finance plugins.

use std::collections::BTreeMap;

use chrono::Utc;
use maud::{Markup, html};
use serde::Deserialize;

use crate::components::{MainContentKey, SwapKey};
use crate::plugins::finance_common::fiscal_year::FiscalYear;

/// Parsed Lariv `environment` JSON cookie (forward-compatible via [`Self::values`]).
#[derive(Debug, Default, Deserialize)]
pub struct LarivEnvironment {
    #[serde(flatten)]
    pub values: BTreeMap<String, String>,
}

impl LarivEnvironment {
    pub fn from_cookie_header(cookie_raw: Option<&str>) -> Self {
        let Some(raw) = cookie_raw else {
            return Self::default();
        };
        for part in raw.split(';') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("environment=") {
                let decoded = percent_decode(val);
                if let Ok(env) = serde_json::from_str::<Self>(&decoded) {
                    return env;
                }
            }
        }
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

#[derive(Clone)]
pub struct FiscalYearOption {
    /// Calendar year of the April 1 FY start.
    pub start_year: i32,
    pub label: String,
}

/// Default filter: current Indian FY (Apr–Mar).
pub fn default_fiscal_year() -> FiscalYear {
    FiscalYear::for_datetime(Utc::now())
}

pub fn list_fiscal_year_options() -> Vec<FiscalYearOption> {
    FiscalYear::options_around(Utc::now(), 5, 1)
        .into_iter()
        .map(|fy| FiscalYearOption {
            start_year: fy.start_year,
            label: fy.label,
        })
        .collect()
}

fn parse_start_year(raw: &str) -> Option<i32> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    raw.parse::<i32>().ok().filter(|&y| y > 0)
}

fn fiscal_year_from_raw(raw: &str) -> Option<FiscalYear> {
    parse_start_year(raw).map(FiscalYear::from_start_year)
}

/// Selected FY start year for the environment dropdown (`None` = explicit "—" / all years).
pub fn selected_fiscal_year_start_for_ui(env: &LarivEnvironment, key: &str) -> Option<i32> {
    match env.get(key) {
        Some(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                None
            } else {
                parse_start_year(raw)
            }
        }
        None => Some(default_fiscal_year().start_year),
    }
}

/// Restrict list queries to a fiscal year window, if the environment selects one.
pub fn resolve_list_fiscal_year(env: &LarivEnvironment, key: &str) -> Option<FiscalYear> {
    match env.get(key) {
        Some(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                None
            } else {
                fiscal_year_from_raw(raw)
            }
        }
        None => Some(default_fiscal_year()),
    }
}

/// Fiscal year dropdown persisted in the `environment` cookie under `cookie_key`.
pub fn fiscal_year_environment_selector(
    cookie_key: &str,
    fiscal_years: &[FiscalYearOption],
    selected_start_year: Option<i32>,
) -> Markup {
    let selected = selected_start_year
        .map(|y| y.to_string())
        .unwrap_or_default();
    let reload_js = format!(
        "htmx.ajax('GET',window.location.pathname+window.location.search,{{target:'{target}',select:'{target}',swap:'outerHTML',pushUrl:false}})",
        target = MainContentKey::SELECTOR,
    );
    let on_change = format!(
        r#"(function(){{
        var env={{}};
        try{{
            var c=document.cookie.split('; ').find(function(r){{return r.startsWith('environment=')}});
            if(c) env=JSON.parse(decodeURIComponent(c.split('=').slice(1).join('=')));
        }}catch(e){{}}
        env[{key:?}]=this.value;
        document.cookie='environment='+encodeURIComponent(JSON.stringify(env))+'; path=/';
        {reload_js};
    }}).call(this)"#,
        key = cookie_key,
    );

    html! {
        div class="my-1 w-full" {
            label class="label text-sm font-bold" { "Fiscal year" }
            select class="select select-bordered w-full" name="fiscal_year" onchange=(on_change) {
                option value="" selected[selected.is_empty()] { "—" }
                @for fy in fiscal_years {
                    option value=(fy.start_year.to_string()) selected[selected == fy.start_year.to_string()] {
                        (fy.label.as_str())
                    }
                }
            }
        }
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "finance_accounts_fiscal_year";

    #[test]
    fn missing_cookie_defaults_to_current_fy() {
        let env = LarivEnvironment::from_cookie_header(None);
        let current = default_fiscal_year().start_year;
        assert_eq!(selected_fiscal_year_start_for_ui(&env, KEY), Some(current));
        assert_eq!(
            resolve_list_fiscal_year(&env, KEY).map(|fy| fy.start_year),
            Some(current)
        );
    }

    #[test]
    fn empty_value_means_all_years() {
        let raw = r#"environment=%7B%22finance_accounts_fiscal_year%22%3A%22%22%7D"#;
        let env = LarivEnvironment::from_cookie_header(Some(raw));
        assert_eq!(selected_fiscal_year_start_for_ui(&env, KEY), None);
        assert!(resolve_list_fiscal_year(&env, KEY).is_none());
    }

    #[test]
    fn explicit_start_year_is_honoured() {
        let env = LarivEnvironment {
            values: BTreeMap::from([(KEY.to_string(), "2024".into())]),
        };
        assert_eq!(selected_fiscal_year_start_for_ui(&env, KEY), Some(2024));
        let fy = resolve_list_fiscal_year(&env, KEY).expect("fy");
        assert_eq!(fy.start_year, 2024);
        assert_eq!(fy.label, "FY 2024-25");
    }
}
