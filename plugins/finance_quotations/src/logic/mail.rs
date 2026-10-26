//! Draft quotation email (`.eml`) with the quotation PDF attached.
//!
//! `X-Unsent: 1` asks Thunderbird and Outlook to open the file as a compose
//! window, with the subject, body, recipient, and PDF already filled in.

use base64::Engine;
use chrono::Utc;
use minijinja::Environment;
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::logic::pdf::{self, PdfError};
use crate::logic::prefs::load_preferences;
use lariv_plugin_filesystem::state::FilesystemState;

pub const DEFAULT_QUOTATION_EMAIL_SUBJECT: &str = "Quotation {{ invoice_number }}";

pub const DEFAULT_QUOTATION_EMAIL_BODY: &str = "\
Dear {{ customer_name }},

Please find attached our quotation {{ invoice_number }} dated {{ date }} for ₹{{ grand_total }}.

Regards,
{{ company_name }}
";

pub struct QuotationEml {
    pub bytes: Vec<u8>,
    pub filename: String,
}

/// Build a draft message for one quotation, with its PDF attached.
pub async fn build_quotation_eml(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    id: i64,
) -> Result<QuotationEml, PdfError> {
    let parts = pdf::render_quotation_pdf_parts(db, fs, id).await?;
    let prefs = load_preferences(db).await;
    let ctx = mail_context(&parts.context);
    let subject_src = prefs
        .quotation_email_subject
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(DEFAULT_QUOTATION_EMAIL_SUBJECT);
    let body_src = prefs
        .quotation_email_body
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(DEFAULT_QUOTATION_EMAIL_BODY);
    let subject = single_line(&render_text(subject_src, &ctx)?);
    let subject = if subject.is_empty() {
        format!("Quotation {}", json_str(&ctx, "invoice_number"))
    } else {
        subject
    };
    let body = render_text(body_src, &ctx)?;
    let to = mail_recipient(&ctx);
    let pdf_name = format!("{}.pdf", parts.pdf.filename_base);
    let filename = format!("{}.eml", parts.pdf.filename_base);
    let bytes = assemble_eml(&EmlParts {
        to: &to,
        subject: &subject,
        body: &body,
        pdf_filename: &pdf_name,
        pdf_bytes: &parts.pdf.bytes,
    });
    Ok(QuotationEml { bytes, filename })
}

fn mail_context(pdf_ctx: &Value) -> Value {
    let invoice_number = json_str_owned(pdf_ctx.get("InvoiceNumber"));
    let date = json_str_owned(pdf_ctx.get("Date"));
    let grand_total = json_str_owned(pdf_ctx.get("GrandTotal"));
    let customer_name = json_str_owned(pdf_ctx.pointer("/Customer/Name"));
    let customer_email = json_str_owned(pdf_ctx.pointer("/Customer/Email"));
    let mut ctx = pdf_ctx.clone();
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert("invoice_number".into(), json!(invoice_number));
        obj.insert("date".into(), json!(date));
        obj.insert("grand_total".into(), json!(grand_total));
        obj.insert("customer_name".into(), json!(customer_name));
        obj.insert("customer_email".into(), json!(customer_email));
    }
    ctx
}

fn json_str_owned(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn json_str(ctx: &Value, key: &str) -> String {
    json_str_owned(ctx.get(key))
}

fn render_text(src: &str, ctx: &Value) -> Result<String, PdfError> {
    let mut env = Environment::new();
    env.add_template("mail", src)
        .map_err(|err| PdfError::Message(format!("Email template: {err}")))?;
    env.get_template("mail")
        .map_err(|err| PdfError::Message(format!("Email template: {err}")))?
        .render(ctx)
        .map_err(|err| PdfError::Message(format!("Email template: {err}")))
}

fn single_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn mail_recipient(ctx: &Value) -> String {
    let raw = json_str(ctx, "customer_email");
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || !trimmed.contains('@')
        || trimmed
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || matches!(ch, '<' | '>' | ',' | ';'))
    {
        String::new()
    } else {
        trimmed.to_string()
    }
}

struct EmlParts<'a> {
    to: &'a str,
    subject: &'a str,
    body: &'a str,
    pdf_filename: &'a str,
    pdf_bytes: &'a [u8],
}

fn assemble_eml(parts: &EmlParts<'_>) -> Vec<u8> {
    let boundary = format!("=_lariv_{}", Uuid::new_v4().simple());
    let mut msg = String::new();
    msg.push_str(&format!(
        "Date: {}\r\n",
        Utc::now().format("%a, %d %b %Y %H:%M:%S +0000")
    ));
    if !parts.to.is_empty() {
        msg.push_str(&format!("To: {}\r\n", parts.to));
    }
    msg.push_str(&format!("Subject: {}\r\n", encode_subject(parts.subject)));
    msg.push_str("MIME-Version: 1.0\r\n");
    msg.push_str("X-Unsent: 1\r\n");
    msg.push_str(&format!(
        "Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n\r\n"
    ));
    msg.push_str(&format!("--{boundary}\r\n"));
    msg.push_str("Content-Type: text/plain; charset=\"UTF-8\"\r\n");
    msg.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
    msg.push_str(&base64_lines(crlf(parts.body).as_bytes()));
    msg.push_str(&format!("--{boundary}\r\n"));
    let pdf_name = quoted_filename(parts.pdf_filename);
    msg.push_str(&format!(
        "Content-Type: application/pdf; name=\"{pdf_name}\"\r\n"
    ));
    msg.push_str("Content-Transfer-Encoding: base64\r\n");
    msg.push_str(&format!(
        "Content-Disposition: attachment; filename=\"{pdf_name}\"\r\n\r\n"
    ));
    msg.push_str(&base64_lines(parts.pdf_bytes));
    msg.push_str(&format!("--{boundary}--\r\n"));
    msg.into_bytes()
}

fn crlf(s: &str) -> String {
    let out = s
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n");
    if out.ends_with("\r\n") {
        out
    } else {
        format!("{out}\r\n")
    }
}

fn quoted_filename(name: &str) -> String {
    name.replace(['\\', '"', '\r', '\n'], "-")
}

fn encode_subject(subject: &str) -> String {
    if subject.is_ascii() && !subject.chars().any(|ch| matches!(ch, '"' | '\\')) {
        return subject.to_string();
    }
    let bytes = subject.as_bytes();
    let mut out = String::new();
    for chunk in bytes.chunks(45) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str("=?UTF-8?B?");
        out.push_str(&base64_inline(chunk));
        out.push_str("?=");
    }
    out
}

fn base64_inline(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_lines(data: &[u8]) -> String {
    let encoded = base64_inline(data);
    let mut out = String::new();
    let mut col = 0;
    for ch in encoded.chars() {
        if col == 76 {
            out.push_str("\r\n");
            col = 0;
        }
        out.push(ch);
        col += 1;
    }
    if col != 0 {
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_subject_renders_invoice_number() {
        let ctx = json!({"invoice_number": "QT-9", "customer_name": "Acme"});
        let rendered =
            render_text(DEFAULT_QUOTATION_EMAIL_SUBJECT, &ctx).expect("default subject renders");
        assert_eq!(rendered, "Quotation QT-9");
    }

    #[test]
    fn eml_is_an_unsent_draft_with_pdf() {
        let eml = assemble_eml(&EmlParts {
            to: "billing@example.com",
            subject: "Quotation QT-9",
            body: "Hello ₹",
            pdf_filename: "QT-9.pdf",
            pdf_bytes: b"%PDF-1.7",
        });
        let text = String::from_utf8(eml).expect("eml is utf-8");
        assert!(text.contains("X-Unsent: 1\r\n"));
        assert!(text.contains("To: billing@example.com\r\n"));
        assert!(text.contains("Subject: Quotation QT-9\r\n"));
        assert!(text.contains("filename=\"QT-9.pdf\""));
        assert!(text.contains(&base64_inline(b"%PDF-1.7")));
        assert!(text.contains(&base64_inline("Hello ₹\r\n".as_bytes())));
    }

    #[test]
    fn non_ascii_subject_is_encoded() {
        let encoded = encode_subject("Quotation ₹ 1");
        assert!(encoded.starts_with("=?UTF-8?B?"));
        assert!(!encoded.contains('₹'));
    }

    #[test]
    fn unsafe_recipient_is_omitted() {
        let ctx = json!({"customer_email": "bad\r\nBcc: x@y"});
        assert!(mail_recipient(&ctx).is_empty());
        let ctx = json!({"customer_email": "ok@example.com"});
        assert_eq!(mail_recipient(&ctx), "ok@example.com");
    }
}
