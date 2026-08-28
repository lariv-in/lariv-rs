//! Sanitization helpers for Gemini Chat.validateContent compatibility
//! and PostgreSQL jsonb/text storage.

use std::borrow::Cow;

#[cfg(test)]
use crate::genai::part_is_empty;
use crate::genai::{Content, Part};
use serde_json::Value;

/// Zero-width space used when a part would otherwise fail Chat validateContent.
pub const ZWSP: &str = "\u{200b}";

/// Strip NUL (`U+0000`) so values can be stored in Postgres text/jsonb.
///
/// Postgres jsonb rejects `\u0000` escapes; text columns also cannot hold NUL.
pub fn strip_nul_chars(s: &str) -> Cow<'_, str> {
    if !s.as_bytes().contains(&0) {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(s.chars().filter(|&c| c != '\0').collect())
    }
}

/// Recursively strip NUL from JSON strings (and object keys).
pub fn sanitize_json_for_postgres(value: Value) -> Value {
    match value {
        Value::String(s) => Value::String(strip_nul_chars(&s).into_owned()),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(sanitize_json_for_postgres).collect())
        }
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| {
                    (
                        strip_nul_chars(&k).into_owned(),
                        sanitize_json_for_postgres(v),
                    )
                })
                .collect(),
        ),
        other => other,
    }
}

pub fn sanitize_json_opt(value: Option<Value>) -> Option<Value> {
    value.map(sanitize_json_for_postgres)
}

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

/// Replace `inline_data` / `file_data` parts with a short text stub.
///
/// Used on tool-loop follow-up `generateContent` calls so Gemini does not
/// re-process large attachments every round. DB / UI history keep the originals.
pub fn elide_attachment_parts_for_api(contents: &mut [Content]) {
    for content in contents.iter_mut() {
        for part in &mut content.parts {
            let label = if !part.display_name.is_empty() {
                part.display_name.as_str()
            } else {
                "attachment"
            };
            if part.inline_data.is_some() {
                part.inline_data = None;
                part.text = Some(format!(
                    "[Attachment \"{label}\" omitted from follow-up request; already provided earlier]"
                ));
            } else if part.file_data.is_some() {
                part.file_data = None;
                part.text = Some(format!(
                    "[Attachment \"{label}\" omitted from follow-up request; already provided earlier]"
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genai::{Blob, FileData, Part, Role};

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

    #[test]
    fn elide_replaces_file_and_inline_parts() {
        let mut contents = vec![Content {
            role: Role::User,
            parts: vec![
                Part {
                    text: Some("hi".into()),
                    ..Default::default()
                },
                Part {
                    file_data: Some(FileData {
                        file_uri: "https://example/files/1".into(),
                        mime_type: "application/pdf".into(),
                    }),
                    display_name: "doc.pdf".into(),
                    ..Default::default()
                },
                Part {
                    inline_data: Some(Blob {
                        mime_type: "image/png".into(),
                        data: "abc".into(),
                    }),
                    display_name: "pic.png".into(),
                    ..Default::default()
                },
            ],
        }];
        elide_attachment_parts_for_api(&mut contents);
        assert!(contents[0].parts[1].file_data.is_none());
        assert!(contents[0].parts[2].inline_data.is_none());
        assert!(
            contents[0].parts[1]
                .text
                .as_deref()
                .unwrap_or("")
                .contains("doc.pdf")
        );
        assert!(
            contents[0].parts[2]
                .text
                .as_deref()
                .unwrap_or("")
                .contains("pic.png")
        );
        assert_eq!(contents[0].parts[0].text.as_deref(), Some("hi"));
    }

    #[test]
    fn strip_nul_chars_removes_nulls() {
        assert_eq!(strip_nul_chars("ok"), "ok");
        assert_eq!(strip_nul_chars("a\0b\0c"), "abc");
    }

    #[test]
    fn sanitize_json_strips_nested_nuls() {
        let v = serde_json::json!({
            "content": "hello\0world",
            "nested": { "x\0y": ["a\0", 1, null] }
        });
        let clean = sanitize_json_for_postgres(v);
        assert_eq!(clean["content"], "helloworld");
        assert_eq!(clean["nested"]["xy"][0], "a");
        assert_eq!(clean["nested"]["xy"][1], 1);
    }
}
