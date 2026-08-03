//! Sanitization helpers for Gemini Chat.validateContent compatibility.

use crate::genai::{Content, Part};

/// Zero-width space used when a part would otherwise fail Chat validateContent.
pub const ZWSP: &str = "\u{200b}";

/// True for a non-nil Part whose only “content” would still be ignored by the API.
pub fn genai_part_is_empty(part: &Part) -> bool {
    crate::genai::part_is_empty(part)
}

/// Mirrors google.golang.org/genai validateContent per-part logic.
pub fn genai_part_passes_chat_validate_content(part: &Part) -> bool {
    if !part.text.as_deref().unwrap_or("").is_empty() {
        return true;
    }
    part.inline_data.is_some()
        || part.file_data.is_some()
        || part.function_call.is_some()
        || part.function_response.is_some()
        || part.executable_code.is_some()
        || part.code_execution_result.is_some()
}

/// Inject ZWSP into parts that would be dropped by Chat curated history.
pub fn sanitize_content_parts_for_genai_chat(content: &mut Content) {
    for part in &mut content.parts {
        if !genai_part_passes_chat_validate_content(part) {
            part.text = Some(ZWSP.to_string());
        }
    }
}

/// Clear InlineData.DisplayName before API calls (kept in DB for UI).
pub fn strip_display_name_from_contents(contents: &mut [Content]) {
    for c in contents {
        strip_display_name_from_parts(&mut c.parts);
    }
}

pub fn strip_display_name_from_parts(parts: &mut [Part]) {
    for p in parts {
        if let Some(blob) = p.inline_data.as_mut() {
            blob.display_name.clear();
        }
        if let Some(fr) = p.function_response.as_mut() {
            for frp in &mut fr.parts {
                if let Some(blob) = frp.inline_data.as_mut() {
                    blob.display_name.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genai::{Blob, Part};

    #[test]
    fn empty_part_is_empty() {
        assert!(genai_part_is_empty(&Part::default()));
    }

    #[test]
    fn thought_only_fails_validate() {
        let p = Part {
            thought: true,
            ..Default::default()
        };
        assert!(!genai_part_passes_chat_validate_content(&p));
        assert!(!genai_part_is_empty(&p));
    }

    #[test]
    fn sanitize_injects_zwsp() {
        let mut c = Content {
            role: "model".into(),
            parts: vec![Part {
                thought: true,
                ..Default::default()
            }],
        };
        sanitize_content_parts_for_genai_chat(&mut c);
        assert_eq!(c.parts[0].text.as_deref(), Some(ZWSP));
    }

    #[test]
    fn strip_display_name() {
        let mut contents = vec![Content {
            role: "user".into(),
            parts: vec![Part {
                inline_data: Some(Blob {
                    mime_type: "image/png".into(),
                    data: "abc".into(),
                    display_name: "x.png".into(),
                }),
                ..Default::default()
            }],
        }];
        strip_display_name_from_contents(&mut contents);
        assert!(
            contents[0].parts[0]
                .inline_data
                .as_ref()
                .unwrap()
                .display_name
                .is_empty()
        );
    }
}
