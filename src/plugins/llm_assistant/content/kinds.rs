//! Part kind strings + IsPartType matching (concrete payloads before text catch-all).

use super::sanitize::genai_part_is_empty;
use crate::genai::Part;

pub const KIND_INLINE_DATA: &str = "inlineData";
pub const KIND_FILE_DATA: &str = "fileData";
pub const KIND_FUNCTION_CALL: &str = "functionCall";
pub const KIND_FUNCTION_RESPONSE: &str = "functionResponse";
pub const KIND_EXECUTABLE_CODE: &str = "executableCode";
pub const KIND_CODE_EXECUTION_RESULT: &str = "codeExecutionResult";
pub const KIND_TOOL_CALL: &str = "toolCall";
pub const KIND_TOOL_RESPONSE: &str = "toolResponse";
pub const KIND_MEDIA_RESOLUTION: &str = "mediaResolution";
pub const KIND_TEXT: &str = "text";

/// Classify a part into a stored kind. Classification order checks before text catch-all.
pub fn classify_part_kind(part: &Part) -> Option<&'static str> {
    if part.inline_data.is_some() {
        return Some(KIND_INLINE_DATA);
    }
    if part.function_response.is_some() {
        return Some(KIND_FUNCTION_RESPONSE);
    }
    if part.media_resolution.is_some() {
        return Some(KIND_MEDIA_RESOLUTION);
    }
    if part.code_execution_result.is_some() {
        return Some(KIND_CODE_EXECUTION_RESULT);
    }
    if part.executable_code.is_some() {
        return Some(KIND_EXECUTABLE_CODE);
    }
    if part.tool_call.is_some() {
        return Some(KIND_TOOL_CALL);
    }
    if part.file_data.is_some() {
        return Some(KIND_FILE_DATA);
    }
    if part.function_call.is_some() {
        return Some(KIND_FUNCTION_CALL);
    }
    if part.tool_response.is_some() {
        return Some(KIND_TOOL_RESPONSE);
    }
    // text catch-all (Go IsPartType)
    if part.inline_data.is_none()
        && part.file_data.is_none()
        && part.function_call.is_none()
        && part.function_response.is_none()
        && part.code_execution_result.is_none()
        && part.executable_code.is_none()
        && part.media_resolution.is_none()
        && part.tool_call.is_none()
        && part.tool_response.is_none()
        && (!part.text.as_deref().unwrap_or("").is_empty()
            || part.thought
            || !part.thought_signature.as_deref().unwrap_or("").is_empty()
            || genai_part_is_empty(part))
    {
        return Some(KIND_TEXT);
    }
    None
}
