//! Tooltip copy for payment preference fields patched onto `/finance/preferences`.

pub const PAYMENT_NUMBER_FORMAT_HINT: &str = "\
Applied when recording a payment that has no payment number (blank or unset). \
If the payment already has a number, that value is kept unchanged.

This is not a template engine — only these literal placeholders are replaced when the payment is saved:
• {{FISCAL_CODE}} — Indian fiscal year code for the payment date (Apr–Mar, e.g. 24-25)
• {{YY}} — two-digit year of the payment date (e.g. 26)
• {{YYYY}} — four-digit year (e.g. 2026)
• {{POSTED_SEQ}} — next payments row id (MAX(id)+1 among live rows), not a per-year sequence counter
• {{FISCAL_POSTED_SEQ}} — count of payments whose payment date falls in the same Indian fiscal year as this payment, plus one (resets each Apr–Mar FY)

Leave blank to default to PAY/{{FISCAL_CODE}}/{{POSTED_SEQ}}.
Example: PAY-{{YYYY}}-{{POSTED_SEQ}}";

pub const PAYMENT_NUMBER_HINT: &str = "\
Leave blank to assign the next number from Payment number format.";
