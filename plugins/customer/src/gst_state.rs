//! GST place-of-supply codes stored on `customers.state`.

/// `(code, "code — NAME")` for the customer state select.
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

/// Blank becomes `None`. Any other value must be a code from [`GST_STATE_CHOICES`].
pub fn parse_state(raw: &str) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if GST_STATE_CHOICES.iter().any(|(code, _)| *code == trimmed) {
        Ok(Some(trimmed.to_string()))
    } else {
        Err("select a state from the list".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{GST_STATE_CHOICES, parse_state};

    #[test]
    fn state_list_is_the_gst_code_table() {
        let codes: Vec<&str> = GST_STATE_CHOICES.iter().map(|(code, _)| *code).collect();
        assert_eq!(codes.len(), 38);
        assert!(codes.contains(&"1"));
        assert!(codes.contains(&"26"));
        assert!(codes.contains(&"27"));
        assert!(codes.contains(&"38"));
        assert!(codes.contains(&"97"));
        assert!(codes.contains(&"99"));
        assert!(!codes.contains(&"25"));
        assert!(!codes.contains(&"28"));
        assert_eq!(
            GST_STATE_CHOICES
                .iter()
                .find(|(code, _)| *code == "27")
                .map(|(_, label)| *label),
            Some("27 — MAHARASHTRA")
        );
    }

    #[test]
    fn parse_state_accepts_codes_and_blank_only() {
        assert!(matches!(parse_state("  "), Ok(None)));
        assert!(matches!(parse_state("27"), Ok(Some(code)) if code == "27"));
        assert!(matches!(parse_state(" 7 "), Ok(Some(code)) if code == "7"));
        assert!(parse_state("25").is_err());
        assert!(parse_state("MAHARASHTRA").is_err());
        assert!(parse_state("27-MAHARASHTRA").is_err());
    }
}
