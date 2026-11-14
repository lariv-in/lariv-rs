//! GST place-of-supply codes stored on `crm_companies.state`.

/// `(code, "code — NAME")` for the company state select.
pub const GST_STATE_CHOICES: &[(&str, &str)] = &[
    ("1", "1 — JAMMU AND KASHMIR"),
    ("2", "2 — HIMACHAL PRADESH"),
    ("3", "3 — PUNJAB"),
    ("4", "4 — CHANDIGARH"),
    ("5", "5 — UTTARAKHAND"),
    ("6", "6 — HARYANA"),
    ("7", "7 — DELHI"),
    ("8", "8 — RAJASTHAN"),
    ("9", "9 — UTTAR PRADESH"),
    ("10", "10 — BIHAR"),
    ("11", "11 — SIKKIM"),
    ("12", "12 — ARUNACHAL PRADESH"),
    ("13", "13 — NAGALAND"),
    ("14", "14 — MANIPUR"),
    ("15", "15 — MIZORAM"),
    ("16", "16 — TRIPURA"),
    ("17", "17 — MEGHALAYA"),
    ("18", "18 — ASSAM"),
    ("19", "19 — WEST BENGAL"),
    ("20", "20 — JHARKHAND"),
    ("21", "21 — ORISSA"),
    ("22", "22 — CHHATTISGARH"),
    ("23", "23 — MADHYA PRADESH"),
    ("24", "24 — GUJARAT"),
    ("26", "26 — DADRA AND NAGAR HAVELI & DAMAN AND DIU"),
    ("27", "27 — MAHARASHTRA"),
    ("29", "29 — KARNATAKA"),
    ("30", "30 — GOA"),
    ("31", "31 — LAKSHADWEEP"),
    ("32", "32 — KERALA"),
    ("33", "33 — TAMIL NADU"),
    ("34", "34 — PUDUCHERRY"),
    ("35", "35 — ANDAMAN AND NICOBAR"),
    ("36", "36 — TELANGANA"),
    ("37", "37 — ANDHRA PRADESH"),
    ("38", "38 — LADAKH"),
    ("97", "97 — OTHER TERRITORY"),
    ("99", "99 — OTHER COUNTRY"),
];

const ALIASES: &[(&str, &str)] = &[
    ("ODISHA", "21"),
    ("PONDICHERRY", "34"),
    ("ANDAMAN AND NICOBAR ISLANDS", "35"),
];

/// Blank becomes `None`. A code, dropdown label, or existing state name becomes
/// the dropdown value, such as `27`.
pub fn parse_state(raw: &str) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    match unpadded_code(trimmed) {
        Some(code) => Ok(Some(code.to_string())),
        None => Err("select a state from the list".to_string()),
    }
}

/// Value for the state dropdown. Known names and codes select that option.
pub fn dropdown_value(raw: &str) -> String {
    let trimmed = raw.trim();
    unpadded_code(trimmed)
        .map(str::to_string)
        .unwrap_or_else(|| trimmed.to_string())
}

/// Two-digit GST code for a stored state or a GSTIN.
///
/// `Karnataka`, `29`, `07`, and `29 — KARNATAKA` all become `29` or `07`.
/// A GSTIN keeps its two-digit prefix even when that prefix is not in the list.
pub fn gst_state_code(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(code) = unpadded_code(trimmed) {
        return Some(pad_code(code));
    }
    let digits: String = trimmed
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take(2)
        .collect();
    if digits.len() != 2 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(unpadded_code(&digits).map(pad_code).unwrap_or(digits))
}

/// State text for the company detail page.
///
/// A known name or GST code becomes `code — NAME`. An unknown name keeps its
/// text and, when the GSTIN starts with a known code, appends that code.
pub fn state_with_code(state: &str, gstin: &str) -> String {
    let trimmed = state.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(label) = label_for(trimmed) {
        return label.to_string();
    }
    if let Some(code) = gstin_state_code(gstin) {
        return format!("{trimmed} ({code})");
    }
    trimmed.to_string()
}

fn label_for(raw: &str) -> Option<&'static str> {
    unpadded_code(raw).and_then(|code| {
        GST_STATE_CHOICES
            .iter()
            .find(|(choice, _)| *choice == code)
            .map(|(_, label)| *label)
    })
}

fn unpadded_code(raw: &str) -> Option<&'static str> {
    if let Some((code, _)) = GST_STATE_CHOICES
        .iter()
        .find(|(_, label)| label.eq_ignore_ascii_case(raw))
    {
        return Some(*code);
    }
    if let Some(code) = canonical_code(raw) {
        return Some(code);
    }
    let name = normalize_name(raw);
    if name.is_empty() {
        return None;
    }
    if let Some((code, _)) = GST_STATE_CHOICES
        .iter()
        .find(|(_, label)| normalize_name(state_name(label)) == name)
    {
        return Some(*code);
    }
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, code)| *code)
}

fn canonical_code(digits: &str) -> Option<&'static str> {
    if digits.is_empty() || digits.len() > 2 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let parsed = digits.parse::<u8>().ok()?;
    GST_STATE_CHOICES
        .iter()
        .find(|(code, _)| code.parse::<u8>().ok() == Some(parsed))
        .map(|(code, _)| *code)
}

fn gstin_state_code(gstin: &str) -> Option<&'static str> {
    let trimmed = gstin.trim();
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    let second = chars.next()?;
    if !first.is_ascii_digit() || !second.is_ascii_digit() {
        return None;
    }
    let prefix = format!("{first}{second}");
    canonical_code(&prefix)
}

fn state_name(label: &str) -> &str {
    label.split_once(" — ").map_or(label, |(_, name)| name)
}

fn pad_code(code: &str) -> String {
    if code.len() == 1 {
        format!("0{code}")
    } else {
        code.to_string()
    }
}

fn normalize_name(raw: &str) -> String {
    let expanded = raw.replace('&', " AND ");
    expanded
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{dropdown_value, gst_state_code, parse_state, state_with_code};

    #[test]
    fn known_name_or_code_includes_the_gst_code() {
        assert_eq!(state_with_code("Karnataka", ""), "29 — KARNATAKA");
        assert_eq!(state_with_code(" 29 ", ""), "29 — KARNATAKA");
        assert_eq!(state_with_code("07", ""), "7 — DELHI");
        assert_eq!(state_with_code("odisha", ""), "21 — ORISSA");
        assert_eq!(
            state_with_code("Dadra and Nagar Haveli and Daman and Diu", ""),
            "26 — DADRA AND NAGAR HAVELI & DAMAN AND DIU"
        );
    }

    #[test]
    fn unknown_state_uses_the_gstin_prefix_when_it_is_a_code() {
        assert_eq!(state_with_code("  ", "29ABCDE1234F1Z5"), "");
        assert_eq!(
            state_with_code("Custom Place", "27ABCDE1234F1Z5"),
            "Custom Place (27)"
        );
        assert_eq!(
            state_with_code("Custom Place", "not-a-gstin"),
            "Custom Place"
        );
    }

    #[test]
    fn parse_state_stores_the_dropdown_code() {
        assert!(matches!(parse_state("  "), Ok(None)));
        assert!(matches!(parse_state("27"), Ok(Some(code)) if code == "27"));
        assert!(matches!(parse_state(" 7 "), Ok(Some(code)) if code == "7"));
        assert!(matches!(parse_state("Maharashtra"), Ok(Some(code)) if code == "27"));
        assert!(parse_state("25").is_err());
        assert!(parse_state("Atlantis").is_err());
    }

    #[test]
    fn export_code_accepts_names_codes_and_gstin() {
        assert_eq!(gst_state_code("Karnataka").as_deref(), Some("29"));
        assert_eq!(gst_state_code("07").as_deref(), Some("07"));
        assert_eq!(gst_state_code("27AAAAA0000A1Z5").as_deref(), Some("27"));
        assert_eq!(gst_state_code("25AAAAA0000A1Z5").as_deref(), Some("25"));
        assert_eq!(dropdown_value(" maharashtra "), "27");
        assert_eq!(gst_state_code("").as_deref(), None);
    }
}
