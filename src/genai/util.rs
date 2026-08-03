//! Helpers for Gemini Content/Part handling.
//!
//! Utilities for extracting display text, detecting empty parts, and merging SSE streaming
//! chunks into a single [`Content`].
//!
//! # Functions
//!
//! - [`part_is_empty`] — skip no-op parts when merging
//! - [`content_text`] — concatenate text parts for display
//! - [`merge_content`] — accumulate streaming deltas

use super::types::{Content, Part, ROLE_MODEL};

/// True for a non-nil Part whose only "content" would still be ignored by the API.
pub fn part_is_empty(part: &Part) -> bool {
    part.media_resolution.is_none()
        && part.code_execution_result.is_none()
        && part.executable_code.is_none()
        && part.file_data.is_none()
        && part.function_call.is_none()
        && part.function_response.is_none()
        && part.inline_data.is_none()
        && part.text.as_deref().unwrap_or("").is_empty()
        && !part.thought
        && part.thought_signature.as_deref().unwrap_or("").is_empty()
        && part.video_metadata.is_none()
        && part.tool_call.is_none()
        && part.tool_response.is_none()
        && part.part_metadata.is_none()
}

/// Concatenate all text parts from a [`Content`] response (for display / one-shot extraction).
pub fn content_text(content: &Content) -> String {
    content
        .parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

/// Merge streaming Content chunks.
pub fn merge_content(dst: Option<Content>, src: Content) -> Content {
    let mut src = src;
    if src.role.trim().is_empty() {
        src.role = ROLE_MODEL.to_string();
    }
    let Some(mut dst) = dst else {
        let parts = src
            .parts
            .into_iter()
            .filter(|p| !part_is_empty(p))
            .collect();
        return Content {
            role: src.role,
            parts,
        };
    };
    if dst.role.trim().is_empty() {
        dst.role = src.role;
    }
    for p in src.parts {
        if !part_is_empty(&p) {
            dst.parts.push(p);
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genai::{Blob, Part};

    #[test]
    fn empty_part_is_empty() {
        assert!(part_is_empty(&Part::default()));
    }

    #[test]
    fn text_part_not_empty() {
        let p = Part {
            text: Some("hi".into()),
            ..Default::default()
        };
        assert!(!part_is_empty(&p));
    }

    #[test]
    fn content_text_joins_parts() {
        let c = Content {
            role: "model".into(),
            parts: vec![
                Part {
                    text: Some("hello ".into()),
                    ..Default::default()
                },
                Part {
                    text: Some("world".into()),
                    ..Default::default()
                },
            ],
        };
        assert_eq!(content_text(&c), "hello world");
    }
}
