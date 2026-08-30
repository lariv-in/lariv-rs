//! RFC822 MIME parsing for inbound email automation.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Cursor;

use mailparse::{MailAddr, MailHeaderMap, ParsedMail, addrparse, parse_mail};

use super::config::{
    EMAIL_MAX_ATTACHMENT_BYTES, EMAIL_MAX_ATTACHMENTS, EMAIL_MAX_TOTAL_ATTACHMENT_BYTES,
};

const LOG_TARGET: &str = "llm_assistant::imap";

/// One attachment extracted during MIME parse (held in memory until filter passes).
#[derive(Debug, Clone)]
pub struct ParsedAttachment {
    pub filename: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

/// Structured inbound email after MIME parse.
#[derive(Debug, Clone)]
pub struct ParsedEmail {
    pub message_id: Option<String>,
    pub reply_to: String,
    pub references: Option<String>,
    pub subject: String,
    pub body_text: String,
    pub attachments: Vec<ParsedAttachment>,
    pub from_display: String,
    pub date: Option<String>,
}

impl ParsedEmail {
    /// Dedup key: `Message-ID` when present, else synthetic hash.
    pub fn dedup_key(&self, uid: u32, fallback_from: &str, fallback_subject: &str) -> String {
        if let Some(id) = self
            .message_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return normalize_message_id(id);
        }
        let mut h = DefaultHasher::new();
        uid.hash(&mut h);
        self.date.hash(&mut h);
        fallback_from.hash(&mut h);
        fallback_subject.hash(&mut h);
        self.from_display.hash(&mut h);
        format!("synthetic:{:x}", h.finish())
    }
}

fn normalize_message_id(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('<') && trimmed.ends_with('>') {
        trimmed.to_string()
    } else {
        format!("<{trimmed}>")
    }
}

/// Parse raw RFC822 bytes into structured fields and in-memory attachments.
pub fn parse_rfc822(
    raw: &[u8],
    envelope_from: &str,
    envelope_subject: &str,
) -> anyhow::Result<ParsedEmail> {
    let mail = parse_mail(raw).map_err(|e| anyhow::anyhow!("MIME parse: {e}"))?;

    let message_id = header_value(&mail, "Message-ID").map(|id| normalize_message_id(&id));
    let references = header_value(&mail, "References");
    let subject = header_value(&mail, "Subject").unwrap_or_else(|| envelope_subject.to_string());
    let date = header_value(&mail, "Date");

    let from_display = header_value(&mail, "From").unwrap_or_else(|| envelope_from.to_string());
    let reply_to = parse_reply_address(&mail).unwrap_or_else(|| from_display.clone());

    let mut plain_body: Option<String> = None;
    let mut html_body: Option<String> = None;
    let mut attachments = Vec::new();
    let mut total_attachment_bytes = 0usize;

    walk_parts(
        &mail,
        &mut plain_body,
        &mut html_body,
        &mut attachments,
        &mut total_attachment_bytes,
    );

    let body_text = match plain_body {
        Some(text) if !text.trim().is_empty() => text,
        _ => html_body
            .map(|html| html_to_plain(&html))
            .unwrap_or_default(),
    };

    Ok(ParsedEmail {
        message_id,
        reply_to,
        references,
        subject,
        body_text,
        attachments,
        from_display,
        date,
    })
}

fn parse_reply_address(mail: &ParsedMail) -> Option<String> {
    let raw = header_value(mail, "Reply-To").or_else(|| header_value(mail, "From"))?;
    parse_single_address(&raw)
}

fn parse_single_address(raw: &str) -> Option<String> {
    let addrs = addrparse(raw).ok()?;
    for entry in addrs.iter() {
        match entry {
            MailAddr::Single(s) if !s.addr.trim().is_empty() => {
                let addr = s.addr.trim();
                return Some(
                    match s
                        .display_name
                        .as_deref()
                        .map(str::trim)
                        .filter(|n| !n.is_empty())
                    {
                        Some(name) => format!("{name} <{addr}>"),
                        None => addr.to_string(),
                    },
                );
            }
            MailAddr::Single(_) => {}
            MailAddr::Group(g) => {
                if let Some(s) = g.addrs.first() {
                    let addr = s.addr.trim();
                    if !addr.is_empty() {
                        return Some(
                            match s
                                .display_name
                                .as_deref()
                                .map(str::trim)
                                .filter(|n| !n.is_empty())
                            {
                                Some(name) => format!("{name} <{addr}>"),
                                None => addr.to_string(),
                            },
                        );
                    }
                }
            }
        }
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn header_value(mail: &ParsedMail, name: &str) -> Option<String> {
    mail.headers
        .get_first_header(name)
        .map(|h| h.get_value().trim().to_string())
        .filter(|v| !v.is_empty())
}

fn walk_parts(
    mail: &ParsedMail,
    plain_body: &mut Option<String>,
    html_body: &mut Option<String>,
    attachments: &mut Vec<ParsedAttachment>,
    total_attachment_bytes: &mut usize,
) {
    if !mail.subparts.is_empty() {
        for part in &mail.subparts {
            walk_parts(
                part,
                plain_body,
                html_body,
                attachments,
                total_attachment_bytes,
            );
        }
        return;
    }

    let ctype = mail.ctype.mimetype.to_ascii_lowercase();
    if is_attachment_part(mail, &ctype) {
        try_push_attachment(mail, &ctype, attachments, total_attachment_bytes);
        return;
    }

    if ctype == "text/plain" && plain_body.is_none() {
        if let Ok(text) = mail.get_body() {
            if !text.trim().is_empty() {
                *plain_body = Some(text);
            }
        }
    } else if ctype == "text/html" && html_body.is_none() {
        if let Ok(html) = mail.get_body() {
            if !html.trim().is_empty() {
                *html_body = Some(html);
            }
        }
    }
}

fn is_attachment_part(mail: &ParsedMail, ctype: &str) -> bool {
    let disposition = header_value(mail, "Content-Disposition")
        .map(|d| d.to_ascii_lowercase())
        .unwrap_or_default();

    if disposition.contains("attachment") {
        return true;
    }
    if disposition.contains("inline") {
        return false;
    }
    if ctype.starts_with("text/") {
        return false;
    }
    part_filename(mail).is_some()
}

fn try_push_attachment(
    mail: &ParsedMail,
    ctype: &str,
    attachments: &mut Vec<ParsedAttachment>,
    total_attachment_bytes: &mut usize,
) {
    if attachments.len() >= EMAIL_MAX_ATTACHMENTS {
        tracing::warn!(
            target: LOG_TARGET,
            "skipping attachment — max count ({EMAIL_MAX_ATTACHMENTS}) reached"
        );
        return;
    }

    let bytes = match mail.get_body_raw() {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(target: LOG_TARGET, "skipping attachment — decode failed: {e}");
            return;
        }
    };

    if bytes.len() > EMAIL_MAX_ATTACHMENT_BYTES {
        tracing::warn!(
            target: LOG_TARGET,
            size = bytes.len(),
            limit = EMAIL_MAX_ATTACHMENT_BYTES,
            "skipping oversized attachment"
        );
        return;
    }

    if *total_attachment_bytes + bytes.len() > EMAIL_MAX_TOTAL_ATTACHMENT_BYTES {
        tracing::warn!(
            target: LOG_TARGET,
            size = bytes.len(),
            limit = EMAIL_MAX_TOTAL_ATTACHMENT_BYTES,
            "skipping attachment — total size limit exceeded"
        );
        return;
    }

    let filename = part_filename(mail).unwrap_or_else(|| "attachment".to_string());
    *total_attachment_bytes += bytes.len();
    attachments.push(ParsedAttachment {
        filename,
        mime_type: ctype.to_string(),
        bytes,
    });
}

fn part_filename(mail: &ParsedMail) -> Option<String> {
    if let Some(cd) = header_value(mail, "Content-Disposition") {
        if let Some(name) = parse_filename_param(&cd) {
            return Some(name);
        }
    }
    if let Some(name) = mail.ctype.params.get("name") {
        let trimmed = name.trim().trim_matches('"');
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn parse_filename_param(header: &str) -> Option<String> {
    for segment in header.split(';') {
        let segment = segment.trim();
        let lower = segment.to_ascii_lowercase();
        if lower.starts_with("filename*=") {
            let value = segment.split_once('=')?.1.trim().trim_matches('"');
            if let Some((_charset, encoded)) = value.split_once("''") {
                return Some(encoded.to_string());
            }
        } else if lower.starts_with("filename=") {
            let value = segment.split_once('=')?.1.trim().trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn html_to_plain(html: &str) -> String {
    html2text::from_read(Cursor::new(html.as_bytes()), 80).unwrap_or_else(|_| html.to_string())
}

/// Attachment metadata for the filter LLM (no binary).
pub fn attachment_metadata_lines(attachments: &[ParsedAttachment]) -> String {
    if attachments.is_empty() {
        return String::new();
    }
    attachments
        .iter()
        .map(|a| {
            format!(
                "- {} ({}, {} bytes)",
                a.filename,
                a.mime_type,
                a.bytes.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_plain_text() {
        let raw = b"From: alice@example.com\r\nSubject: Hello\r\nMessage-ID: <abc@example.com>\r\n\r\nHello world";
        let parsed = parse_rfc822(raw, "alice@example.com", "Hello").unwrap();
        assert_eq!(parsed.body_text.trim(), "Hello world");
        assert_eq!(parsed.message_id.as_deref(), Some("<abc@example.com>"));
        assert!(parsed.attachments.is_empty());
    }

    #[test]
    fn reply_to_preferred() {
        let raw = b"From: alice@example.com\r\nReply-To: bot-replies@example.com\r\n\r\nBody";
        let parsed = parse_rfc822(raw, "alice@example.com", "Hi").unwrap();
        assert!(parsed.reply_to.contains("bot-replies@example.com"));
    }

    #[test]
    fn dedup_key_uses_message_id() {
        let parsed = ParsedEmail {
            message_id: Some("<id@test>".into()),
            reply_to: String::new(),
            references: None,
            subject: String::new(),
            body_text: String::new(),
            attachments: vec![],
            from_display: String::new(),
            date: None,
        };
        assert_eq!(parsed.dedup_key(1, "a", "b"), "<id@test>");
    }

    #[test]
    fn dedup_key_synthetic_when_missing_message_id() {
        let parsed = ParsedEmail {
            message_id: None,
            reply_to: String::new(),
            references: None,
            subject: "s".into(),
            body_text: String::new(),
            attachments: vec![],
            from_display: "a@b".into(),
            date: Some("Mon".into()),
        };
        let k1 = parsed.dedup_key(5, "a@b", "s");
        let k2 = parsed.dedup_key(5, "a@b", "s");
        assert!(k1.starts_with("synthetic:"));
        assert_eq!(k1, k2);
    }

    #[test]
    fn html_only_body() {
        let raw = concat!(
            "From: a@b.com\r\n",
            "Subject: HTML\r\n",
            "Content-Type: text/html; charset=utf-8\r\n",
            "\r\n",
            "<html><body><p>Hello <b>world</b></p></body></html>"
        );
        let parsed = parse_rfc822(raw.as_bytes(), "a@b.com", "HTML").unwrap();
        assert!(parsed.body_text.contains("Hello"));
        assert!(parsed.body_text.contains("world"));
    }

    #[test]
    fn base64_attachment_extracted() {
        let raw = concat!(
            "From: a@b.com\r\n",
            "Subject: file\r\n",
            "Content-Type: multipart/mixed; boundary=abc\r\n",
            "\r\n",
            "--abc\r\n",
            "Content-Type: text/plain\r\n",
            "\r\n",
            "See attached\r\n",
            "--abc\r\n",
            "Content-Type: application/octet-stream\r\n",
            "Content-Disposition: attachment; filename=\"data.bin\"\r\n",
            "Content-Transfer-Encoding: base64\r\n",
            "\r\n",
            "AQID\r\n",
            "--abc--\r\n"
        );
        let parsed = parse_rfc822(raw.as_bytes(), "a@b.com", "file").unwrap();
        assert_eq!(parsed.attachments.len(), 1);
        assert_eq!(parsed.attachments[0].filename, "data.bin");
        assert_eq!(parsed.attachments[0].bytes, vec![1, 2, 3]);
    }

    #[test]
    fn attachment_count_limit_skips_excess() {
        let mut parts = String::from(
            "From: a@b.com\r\nSubject: many\r\nContent-Type: multipart/mixed; boundary=b\r\n\r\n",
        );
        for i in 0..12 {
            parts.push_str("--b\r\n");
            parts.push_str("Content-Type: application/octet-stream\r\n");
            parts.push_str(&format!(
                "Content-Disposition: attachment; filename=\"f{i}.bin\"\r\n\r\n"
            ));
            parts.push('x');
            parts.push_str("\r\n");
        }
        parts.push_str("--b--\r\n");
        let parsed = parse_rfc822(parts.as_bytes(), "a@b.com", "many").unwrap();
        assert_eq!(
            parsed.attachments.len(),
            crate::plugins::llm_assistant::config::EMAIL_MAX_ATTACHMENTS
        );
    }
}
