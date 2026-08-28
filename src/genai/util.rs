//! Helpers for Gemini Content/Part handling.
//!
//! Utilities for extracting display text, detecting empty parts, and merging SSE streaming
//! chunks into a single [`Content`].
//!
//! # Functions
//!
//! - [`part_is_empty`] — skip no-op parts when merging
//! - [`content_text`] — concatenate text parts for display
//! - [`content_answer_text`] — text without thought parts
//! - [`coerce_json_text`] — strip prose/fences around a JSON value
//! - [`merge_content`] — accumulate streaming deltas

use super::types::{Content, Part};

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

/// Like [`content_text`], but omits parts marked `thought` (model reasoning).
///
/// Use this when parsing structured / JSON answers: thought text is prose and
/// must not be concatenated in front of the real response.
pub fn content_answer_text(content: &Content) -> String {
    content
        .parts
        .iter()
        .filter(|p| !p.thought)
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

/// Best-effort extraction of a JSON value from model output.
///
/// Models sometimes wrap structured answers in prose ("Here is the JSON…") or
/// markdown fences even when `responseMimeType` is `application/json`. Returns
/// the original trimmed text when no valid JSON substring is found.
pub fn coerce_json_text(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return trimmed;
    }
    if json_value_ok(trimmed) {
        return trimmed;
    }
    let candidate = strip_json_fence(trimmed);
    if json_value_ok(candidate) {
        return candidate;
    }
    if let Some(obj) = extract_first_json_object(candidate) {
        if json_value_ok(obj) {
            return obj;
        }
    }
    if let Some(obj) = extract_first_json_object(trimmed) {
        if json_value_ok(obj) {
            return obj;
        }
    }
    trimmed
}

fn json_value_ok(s: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(s).is_ok()
}

fn strip_json_fence(s: &str) -> &str {
    let s = s.trim();
    let Some(rest) = s.strip_prefix("```") else {
        return s;
    };
    let rest = rest
        .strip_prefix("json")
        .or_else(|| rest.strip_prefix("JSON"))
        .unwrap_or(rest)
        .trim_start_matches(['\r', '\n']);
    rest.strip_suffix("```").unwrap_or(rest).trim()
}

fn extract_first_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end < start {
        return None;
    }
    Some(&s[start..=end])
}

/// True when the part is plain UTF-8 text only (safe to concatenate with the previous text part).
fn is_plain_text_part(part: &Part) -> bool {
    part.text.as_ref().is_some_and(|t| !t.is_empty())
        && part.media_resolution.is_none()
        && part.code_execution_result.is_none()
        && part.executable_code.is_none()
        && part.file_data.is_none()
        && part.function_call.is_none()
        && part.function_response.is_none()
        && part.inline_data.is_none()
        && !part.thought
        && part.thought_signature.as_deref().unwrap_or("").is_empty()
        && part.video_metadata.is_none()
        && part.tool_call.is_none()
        && part.tool_response.is_none()
        && part.part_metadata.is_none()
}

/// Merge streaming Content chunks.
///
/// Adjacent plain-text deltas are concatenated into one part so markdown rendering
/// does not wrap each stream token in its own `<p>`.
pub fn merge_content(dst: Option<Content>, src: Content) -> Content {
    let Some(mut dst) = dst else {
        let mut parts: Vec<Part> = Vec::new();
        for p in src.parts.into_iter().filter(|p| !part_is_empty(p)) {
            append_merged_part(&mut parts, p);
        }
        return Content {
            role: src.role,
            parts,
        };
    };
    for p in src.parts {
        if !part_is_empty(&p) {
            append_merged_part(&mut dst.parts, p);
        }
    }
    dst
}

fn append_merged_part(parts: &mut Vec<Part>, p: Part) {
    if is_plain_text_part(&p) {
        if let Some(last) = parts.last_mut() {
            if is_plain_text_part(last) {
                if let (Some(existing), Some(chunk)) = (last.text.as_mut(), p.text.as_deref()) {
                    existing.push_str(chunk);
                    return;
                }
            }
        }
    }
    parts.push(p);
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
            role: Role::Model,
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

    #[test]
    fn content_answer_text_skips_thoughts() {
        let c = Content {
            role: Role::Model,
            parts: vec![
                Part {
                    text: Some("reasoning...".into()),
                    thought: true,
                    ..Default::default()
                },
                Part {
                    text: Some("{\"ok\":true}".into()),
                    ..Default::default()
                },
            ],
        };
        assert_eq!(content_text(&c), "reasoning...{\"ok\":true}");
        assert_eq!(content_answer_text(&c), "{\"ok\":true}");
    }

    #[test]
    fn coerce_json_text_strips_preamble() {
        let raw = "Here is the JSON requested:\n\n{\"act\":true,\"reason\":\"ok\"}";
        assert_eq!(
            coerce_json_text(raw),
            "{\"act\":true,\"reason\":\"ok\"}"
        );
    }

    #[test]
    fn coerce_json_text_strips_fence() {
        let raw = "```json\n{\"pass\":false,\"reason\":\"spam\"}\n```";
        assert_eq!(
            coerce_json_text(raw),
            "{\"pass\":false,\"reason\":\"spam\"}"
        );
    }

    #[test]
    fn coerce_json_text_leaves_plain_json() {
        let raw = "  {\"act\":false,\"reason\":\"no\"}  ";
        assert_eq!(coerce_json_text(raw), "{\"act\":false,\"reason\":\"no\"}");
    }

    #[test]
    fn merge_content_concatenates_adjacent_text() {
        let a = Content {
            role: Role::Model,
            parts: vec![Part {
                text: Some("H".into()),
                ..Default::default()
            }],
        };
        let b = Content {
            role: Role::Model,
            parts: vec![Part {
                text: Some("ello".into()),
                ..Default::default()
            }],
        };
        let merged = merge_content(Some(a), b);
        assert_eq!(merged.parts.len(), 1);
        assert_eq!(merged.parts[0].text.as_deref(), Some("Hello"));
    }
}
