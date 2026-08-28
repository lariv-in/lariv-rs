//! Shared attachment helpers for chat WebSocket and email inbound content.

use base64::{Engine, engine::general_purpose::STANDARD as B64};

use crate::plugins::llm_assistant::genai::{Blob, FileData, GenaiClient, GenaiError, Part};

/// Guess MIME from filename; if unknown/`octet-stream` and bytes are valid UTF-8, use `text/plain`
/// so Gemini accepts text-like attachments (e.g. `.desktop`).
pub fn detect_mime(name: &str, bytes: &[u8]) -> String {
    if let Some(mime) = mime_guess::from_path(name).first() {
        let essence = mime.essence_str();
        if essence != "application/octet-stream" {
            return essence.to_string();
        }
    }
    if looks_like_utf8(bytes) {
        return "text/plain".to_string();
    }
    "application/octet-stream".to_string()
}

/// Prefer a non-generic declared MIME; otherwise sniff from name/bytes.
pub fn resolve_mime(declared: &str, name: &str, bytes: &[u8]) -> String {
    let trimmed = declared.trim();
    if !trimmed.is_empty() && trimmed != "application/octet-stream" {
        return trimmed.to_string();
    }
    detect_mime(name, bytes)
}

fn looks_like_utf8(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
}

/// Build a Gemini `Part` with base64 `inline_data` for an attachment.
pub fn attachment_part(name: &str, bytes: &[u8]) -> Part {
    Part {
        inline_data: Some(Blob {
            mime_type: detect_mime(name, bytes),
            data: B64.encode(bytes),
        }),
        display_name: name.to_string(),
        ..Default::default()
    }
}

/// Build a Gemini `Part` that references an uploaded Files API URI.
pub fn file_data_part(name: &str, mime_type: &str, file_uri: &str) -> Part {
    Part {
        file_data: Some(FileData {
            file_uri: file_uri.to_string(),
            mime_type: mime_type.to_string(),
        }),
        display_name: name.to_string(),
        ..Default::default()
    }
}

/// Upload bytes to the Gemini Files API and return a `file_data` part.
pub async fn upload_attachment_part(
    genai: &GenaiClient,
    name: &str,
    declared_mime: &str,
    bytes: &[u8],
) -> Result<Part, GenaiError> {
    let mime = resolve_mime(declared_mime, name, bytes);
    let uploaded = genai.upload_file(name, &mime, bytes).await?;
    Ok(file_data_part(
        name,
        if uploaded.mime_type.is_empty() {
            &mime
        } else {
            &uploaded.mime_type
        },
        &uploaded.uri,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_utf8_falls_back_to_text_plain() {
        let body = b"[Desktop Entry]\nName=Test\n";
        assert!(looks_like_utf8(body));
        assert_eq!(detect_mime("app.desktop", body), "text/plain");
    }

    #[test]
    fn known_extension_kept() {
        assert_eq!(detect_mime("photo.png", b"not-really-png"), "image/png");
    }

    #[test]
    fn binary_unknown_stays_octet_stream() {
        let body = [0xff, 0xfe, 0x00, 0x01];
        assert!(!looks_like_utf8(&body));
        assert_eq!(detect_mime("blob.dat", &body), "application/octet-stream");
    }

    #[test]
    fn resolve_mime_keeps_declared() {
        assert_eq!(
            resolve_mime("application/pdf", "x.bin", b"%PDF"),
            "application/pdf"
        );
    }

    #[test]
    fn file_data_part_sets_uri() {
        let part = file_data_part("a.pdf", "application/pdf", "https://example/files/1");
        let fd = part.file_data.expect("file_data");
        assert_eq!(fd.file_uri, "https://example/files/1");
        assert_eq!(fd.mime_type, "application/pdf");
        assert_eq!(part.display_name, "a.pdf");
        assert!(part.inline_data.is_none());
    }
}
