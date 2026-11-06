//! Parsed phone numbers (region IN) and the links derived from them.

use phonenumber::PhoneNumber;

/// Parse `phone` as an Indian number. Empty and invalid input is [`None`].
pub fn parse_phone(phone: &str) -> Option<PhoneNumber> {
    let phone = phone.trim();
    if phone.is_empty() {
        return None;
    }
    let parsed = phonenumber::parse(Some(phonenumber::country::IN), phone).ok()?;
    parsed.is_valid().then_some(parsed)
}

/// `tel:` URL for `phone`, using E.164.
pub fn tel_href(phone: &PhoneNumber) -> String {
    format!("tel:{}", phone.format().mode(phonenumber::Mode::E164))
}

/// `https://wa.me/` URL for `phone`, country code plus national number.
pub fn whatsapp_href(phone: &PhoneNumber) -> String {
    format!(
        "https://wa.me/{}{}",
        phone.code().value(),
        phone.national().value()
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_phone, tel_href, whatsapp_href};

    fn indian_mobile() -> phonenumber::PhoneNumber {
        parse_phone("9876543210").expect("valid indian mobile")
    }

    #[test]
    fn tel_href_formats_indian_mobile() {
        assert_eq!(tel_href(&indian_mobile()), "tel:+919876543210");
    }

    #[test]
    fn whatsapp_href_uses_country_and_national() {
        assert_eq!(
            whatsapp_href(&indian_mobile()),
            "https://wa.me/919876543210"
        );
    }

    #[test]
    fn empty_phone_is_none() {
        assert!(parse_phone("").is_none());
        assert!(parse_phone("   ").is_none());
    }
}
