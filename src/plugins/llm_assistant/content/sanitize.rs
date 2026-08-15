//! Sanitization helpers for Gemini Chat.validateContent compatibility.

#[cfg(test)]
use crate::genai::part_is_empty;
use crate::genai::{Content, Part};

/// Zero-width space used when a part would otherwise fail Chat validateContent.
pub const ZWSP: &str = "\u{200b}";

/// Mirrors google.golang.org/genai validateContent per-part logic.
pub(super) fn genai_part_passes_chat_validate_content(part: &Part) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genai::{Part, Role};

    #[test]
    fn empty_part_is_empty() {
        assert!(part_is_empty(&Part::default()));
    }

    #[test]
    fn thought_only_fails_validate() {
        let p = Part {
            thought: true,
            ..Default::default()
        };
        assert!(!genai_part_passes_chat_validate_content(&p));
        assert!(!part_is_empty(&p));
    }

    #[test]
    fn sanitize_injects_zwsp() {
        let mut c = Content {
            role: Role::Model,
            parts: vec![Part {
                thought: true,
                ..Default::default()
            }],
        };
        sanitize_content_parts_for_genai_chat(&mut c);
        assert_eq!(c.parts[0].text.as_deref(), Some(ZWSP));
    }
}
