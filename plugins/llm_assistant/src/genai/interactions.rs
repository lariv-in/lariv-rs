//! Interactions API request/response shapes and Content ↔ step converters.
//!
//! Wire format is snake_case (`POST /v1beta/interactions`). Local chat history
//! remains Gemini [`Content`] / [`Part`] for persistence and UI.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::types::{
    Blob, Content, FileData, FunctionCall, FunctionDeclaration, FunctionResponse, Part, Role,
    UsageMetadata,
};

/// Result of a completed (or requires_action) interaction.
#[derive(Debug, Clone)]
pub struct InteractionResult {
    pub id: String,
    pub content: Content,
    pub usage: Option<UsageMetadata>,
    pub status: String,
}

/// Request body for `POST /v1beta/interactions`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateInteractionRequest {
    pub model: String,
    pub input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<InteractionTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<InteractionGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_interaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InteractionTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

impl From<&FunctionDeclaration> for InteractionTool {
    fn from(decl: &FunctionDeclaration) -> Self {
        Self {
            tool_type: "function".into(),
            name: decl.name.clone(),
            description: decl.description.clone(),
            parameters: decl.parameters.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InteractionGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_summaries: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

/// Top-level interaction resource (non-streaming or `interaction.completed`).
#[derive(Debug, Clone, Deserialize)]
pub struct Interaction {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub steps: Vec<InteractionStep>,
    #[serde(default)]
    pub usage: Option<InteractionUsage>,
    #[serde(default)]
    pub output_text: Option<String>,
    #[serde(default)]
    pub error: Option<InteractionErrorBody>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InteractionErrorBody {
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct InteractionUsage {
    #[serde(default)]
    pub total_input_tokens: Option<i32>,
    #[serde(default)]
    pub total_output_tokens: Option<i32>,
    #[serde(default)]
    pub total_thought_tokens: Option<i32>,
    #[serde(default)]
    pub total_cached_tokens: Option<i32>,
    #[serde(default)]
    pub total_tokens: Option<i32>,
}

impl InteractionUsage {
    pub fn to_usage_metadata(&self) -> UsageMetadata {
        UsageMetadata {
            prompt_token_count: self.total_input_tokens,
            candidates_token_count: self.total_output_tokens,
            thoughts_token_count: self.total_thought_tokens,
            cached_content_token_count: self.total_cached_tokens,
            total_token_count: self.total_tokens,
        }
    }
}

/// One step in an Interaction timeline (polymorphic on `type`).
#[derive(Debug, Clone, Deserialize)]
pub struct InteractionStep {
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<Value>,
    #[serde(default)]
    pub call_id: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub summary: Option<Vec<Value>>,
    #[serde(default)]
    pub content: Option<Vec<Value>>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub is_error: Option<bool>,
}

/// Streaming SSE event payload (`event_type` discriminator).
#[derive(Debug, Clone, Deserialize)]
pub struct InteractionSseEvent {
    #[serde(default)]
    pub event_type: String,
    #[serde(default)]
    pub index: Option<i32>,
    #[serde(default)]
    pub interaction: Option<Interaction>,
    #[serde(default)]
    pub interaction_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub step: Option<InteractionStep>,
    #[serde(default)]
    pub delta: Option<StepDelta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StepDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub content: Option<Value>,
    /// Fragment of function-call arguments (API ref: string on `arguments_delta`).
    #[serde(default)]
    pub arguments: Option<Value>,
    /// Older / alternate field name seen in some SDK samples.
    #[serde(default)]
    pub partial_arguments: Option<String>,
}

/// Map a MIME type to an Interactions content block `type`.
pub fn media_block_type(mime: &str) -> &'static str {
    let m = mime.trim().to_ascii_lowercase();
    if m.starts_with("image/") {
        "image"
    } else if m.starts_with("audio/") {
        "audio"
    } else if m.starts_with("video/") {
        "video"
    } else {
        "document"
    }
}

/// Convert a local [`Part`] into an Interactions content block (or None).
pub fn part_to_content_block(part: &Part) -> Option<Value> {
    if let Some(text) = part.text.as_ref() {
        if part.thought {
            return None;
        }
        return Some(json!({ "type": "text", "text": text }));
    }
    if let Some(blob) = &part.inline_data {
        let t = media_block_type(&blob.mime_type);
        return Some(json!({
            "type": t,
            "mime_type": blob.mime_type,
            "data": blob.data,
        }));
    }
    if let Some(fd) = &part.file_data {
        let t = media_block_type(&fd.mime_type);
        return Some(json!({
            "type": t,
            "mime_type": fd.mime_type,
            "uri": fd.file_uri,
        }));
    }
    None
}

/// Build Interactions `input` for a full conversation window (bootstrap).
///
/// Emits a chronological list of `user_input`, `model_output` / `thought` /
/// `function_call`, and `function_result` steps so the server can reconstruct
/// the transcript without `previous_interaction_id`.
pub fn contents_to_interaction_input(contents: &[Content]) -> Value {
    let mut steps: Vec<Value> = Vec::new();
    for content in contents {
        match content.role {
            Role::User => {
                if let Some(fr_steps) = function_result_steps_from_content(content) {
                    steps.extend(fr_steps);
                } else {
                    let blocks: Vec<Value> = content
                        .parts
                        .iter()
                        .filter_map(part_to_content_block)
                        .collect();
                    if !blocks.is_empty() {
                        steps.push(json!({
                            "type": "user_input",
                            "content": blocks,
                        }));
                    }
                }
            }
            Role::Model => {
                steps.extend(model_content_to_steps(content));
            }
        }
    }
    Value::Array(steps)
}

/// Build input for a chained turn: only the new user message (no function results).
pub fn user_content_to_interaction_input(content: &Content) -> Value {
    if let Some(fr_steps) = function_result_steps_from_content(content) {
        return Value::Array(fr_steps);
    }
    let blocks: Vec<Value> = content
        .parts
        .iter()
        .filter_map(part_to_content_block)
        .collect();
    if blocks.len() == 1 {
        blocks
            .into_iter()
            .next()
            .unwrap_or(json!({ "type": "text", "text": "" }))
    } else {
        json!(blocks)
    }
}

/// Convert a stored model [`Content`] into Interaction steps for bootstrap history.
pub fn model_content_to_steps(content: &Content) -> Vec<Value> {
    let mut steps = Vec::new();
    let mut text_blocks: Vec<Value> = Vec::new();
    let mut thought_text = String::new();
    let mut thought_sig: Option<String> = None;

    for part in &content.parts {
        if part.thought {
            if let Some(t) = part.text.as_ref().filter(|t| !t.is_empty()) {
                thought_text.push_str(t);
            }
            if let Some(sig) = part.thought_signature.as_ref().filter(|s| !s.is_empty()) {
                thought_sig = Some(sig.clone());
            }
            continue;
        }
        if let Some(sig) = part.thought_signature.as_ref().filter(|s| !s.is_empty())
            && thought_sig.is_none()
        {
            thought_sig = Some(sig.clone());
        }
        if let Some(fc) = &part.function_call {
            flush_thought(&mut steps, &mut thought_text, &mut thought_sig);
            flush_model_output(&mut steps, &mut text_blocks);
            let mut step = json!({
                "type": "function_call",
                "name": fc.name,
                "arguments": fc.args.clone().unwrap_or(json!({})),
            });
            if !fc.id.is_empty() {
                step["id"] = json!(fc.id);
            }
            steps.push(step);
            continue;
        }
        if let Some(text) = part.text.as_ref().filter(|t| !t.is_empty()) {
            text_blocks.push(json!({ "type": "text", "text": text }));
        }
    }
    flush_thought(&mut steps, &mut thought_text, &mut thought_sig);
    flush_model_output(&mut steps, &mut text_blocks);
    steps
}

fn flush_thought(steps: &mut Vec<Value>, text: &mut String, sig: &mut Option<String>) {
    if text.is_empty() && sig.is_none() {
        return;
    }
    let mut step = json!({ "type": "thought" });
    if !text.is_empty() {
        step["summary"] = json!([{ "type": "text", "text": text.clone() }]);
        text.clear();
    }
    if let Some(s) = sig.take() {
        step["signature"] = json!(s);
    }
    steps.push(step);
}

fn flush_model_output(steps: &mut Vec<Value>, blocks: &mut Vec<Value>) {
    if blocks.is_empty() {
        return;
    }
    steps.push(json!({
        "type": "model_output",
        "content": std::mem::take(blocks),
    }));
}

/// Extract `function_result` steps from a user Content that holds tool responses.
pub fn function_result_steps_from_content(content: &Content) -> Option<Vec<Value>> {
    let has_fr = content.parts.iter().any(|p| p.function_response.is_some());
    if !has_fr {
        return None;
    }
    let mut steps = Vec::new();
    for part in &content.parts {
        let Some(fr) = &part.function_response else {
            continue;
        };
        let call_id = if !fr.function_response_id.is_empty() {
            fr.function_response_id.clone()
        } else {
            fr.name.clone()
        };
        let result = function_response_to_result(fr);
        steps.push(json!({
            "type": "function_result",
            "call_id": call_id,
            "name": fr.name,
            "result": result,
        }));
    }
    if steps.is_empty() { None } else { Some(steps) }
}

fn function_response_to_result(fr: &FunctionResponse) -> Value {
    let mut blocks: Vec<Value> = Vec::new();
    if let Some(resp) = &fr.response {
        let text = match resp {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        blocks.push(json!({ "type": "text", "text": text }));
    }
    for fp in &fr.parts {
        if let Some(blob) = &fp.inline_data {
            let t = media_block_type(&blob.mime_type);
            blocks.push(json!({
                "type": t,
                "mime_type": blob.mime_type,
                "data": blob.data,
            }));
        } else if let Some(fd) = &fp.file_data {
            let t = media_block_type(&fd.mime_type);
            blocks.push(json!({
                "type": t,
                "mime_type": fd.mime_type,
                "uri": fd.file_uri,
            }));
        }
    }
    if blocks.is_empty() {
        json!([{ "type": "text", "text": "{}" }])
    } else {
        Value::Array(blocks)
    }
}

/// Convert model-generated Interaction steps into a local [`Content`] for persistence/UI.
pub fn steps_to_model_content(steps: &[InteractionStep]) -> Content {
    let mut parts = Vec::new();
    for step in steps {
        match step.step_type.as_str() {
            "thought" => {
                let mut text = String::new();
                if let Some(summary) = &step.summary {
                    for block in summary {
                        if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                            text.push_str(t);
                        }
                    }
                }
                parts.push(Part {
                    text: if text.is_empty() { None } else { Some(text) },
                    thought: true,
                    thought_signature: step.signature.clone(),
                    ..Default::default()
                });
            }
            "model_output" => {
                if let Some(blocks) = &step.content {
                    for block in blocks {
                        if let Some(part) = content_block_to_part(block) {
                            parts.push(part);
                        }
                    }
                }
            }
            "function_call" => {
                let name = step.name.clone().unwrap_or_default();
                let id = step.id.clone().unwrap_or_default();
                parts.push(Part {
                    function_call: Some(FunctionCall {
                        id,
                        name,
                        args: normalize_function_args(step.arguments.clone()),
                        will_continue: None,
                    }),
                    thought_signature: step.signature.clone(),
                    ..Default::default()
                });
            }
            // Echoed user/tool steps are already stored locally.
            "user_input" | "function_result" => {}
            _ => {}
        }
    }
    Content {
        role: Role::Model,
        parts,
    }
}

fn content_block_to_part(block: &Value) -> Option<Part> {
    let block_type = block.get("type")?.as_str()?;
    match block_type {
        "text" => {
            let text = block.get("text")?.as_str()?.to_string();
            Some(Part {
                text: Some(text),
                ..Default::default()
            })
        }
        "image" | "document" | "audio" | "video" => {
            let mime = block
                .get("mime_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(uri) = block.get("uri").and_then(|v| v.as_str()) {
                Some(Part {
                    file_data: Some(FileData {
                        file_uri: uri.to_string(),
                        mime_type: mime,
                    }),
                    ..Default::default()
                })
            } else if let Some(data) = block.get("data").and_then(|v| v.as_str()) {
                Some(Part {
                    inline_data: Some(Blob {
                        mime_type: mime,
                        data: data.to_string(),
                    }),
                    ..Default::default()
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Accumulator for streaming Interaction SSE into a merged [`Content`].
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    pub interaction_id: Option<String>,
    pub status: String,
    pub usage: Option<UsageMetadata>,
    /// Ordered model parts built from step.start / step.delta / step.stop.
    parts: Vec<Part>,
    /// Index of the open step being streamed (`None` after step.stop).
    open_index: Option<usize>,
    /// Step type of the open step.
    open_type: Option<String>,
    /// Buffered argument JSON string for function_call streaming.
    open_args_buf: String,
    open_fc_name: String,
    open_fc_id: String,
}

impl StreamAccumulator {
    pub fn on_event(&mut self, event: &InteractionSseEvent) {
        match event.event_type.as_str() {
            "interaction.created" => {
                if let Some(ix) = &event.interaction {
                    if !ix.id.is_empty() {
                        self.interaction_id = Some(ix.id.clone());
                    }
                    if !ix.status.is_empty() {
                        self.status = ix.status.clone();
                    }
                }
            }
            "interaction.status_update" => {
                if let Some(s) = &event.status {
                    self.status = s.clone();
                }
                if let Some(id) = &event.interaction_id
                    && self.interaction_id.is_none()
                {
                    self.interaction_id = Some(id.clone());
                }
            }
            "step.start" => {
                if let Some(step) = &event.step {
                    self.start_step(step);
                }
            }
            "step.delta" => {
                if let Some(delta) = &event.delta {
                    self.apply_delta(delta);
                }
            }
            "step.stop" => {
                self.finish_open_step();
            }
            "interaction.completed" => {
                if let Some(ix) = &event.interaction {
                    if !ix.id.is_empty() {
                        self.interaction_id = Some(ix.id.clone());
                    }
                    if !ix.status.is_empty() {
                        self.status = ix.status.clone();
                    }
                    if let Some(usage) = &ix.usage {
                        self.usage = Some(usage.to_usage_metadata());
                    }
                    // Streaming `interaction.completed` is often a partial resource
                    // (id/status/usage only). Keep accumulated step deltas unless the
                    // completed payload includes model-generated steps with args.
                    let from_steps = steps_to_model_content(&ix.steps);
                    if interaction_steps_are_authoritative(&from_steps) {
                        self.parts = from_steps.parts;
                        self.open_index = None;
                        self.open_type = None;
                    }
                }
            }
            _ => {}
        }
    }

    fn start_step(&mut self, step: &InteractionStep) {
        self.finish_open_step();
        match step.step_type.as_str() {
            "thought" => {
                self.parts.push(Part {
                    thought: true,
                    thought_signature: step.signature.clone(),
                    text: Some(String::new()),
                    ..Default::default()
                });
                self.open_index = Some(self.parts.len() - 1);
                self.open_type = Some("thought".into());
            }
            "model_output" => {
                self.parts.push(Part {
                    text: Some(String::new()),
                    ..Default::default()
                });
                self.open_index = Some(self.parts.len() - 1);
                self.open_type = Some("model_output".into());
            }
            "function_call" => {
                self.open_fc_name = step.name.clone().unwrap_or_default();
                self.open_fc_id = step.id.clone().unwrap_or_default();
                self.open_args_buf.clear();
                // step.start often ships `arguments: {}` before deltas arrive.
                // Do not seed the buffer with "{}" or later string deltas become
                // invalid JSON like `{}{"location":...}`.
                let initial_args = normalize_function_args(step.arguments.clone());
                if let Some(Value::String(s)) = &step.arguments {
                    self.open_args_buf = s.clone();
                }
                self.parts.push(Part {
                    function_call: Some(FunctionCall {
                        id: self.open_fc_id.clone(),
                        name: self.open_fc_name.clone(),
                        args: initial_args,
                        will_continue: None,
                    }),
                    thought_signature: step.signature.clone(),
                    ..Default::default()
                });
                self.open_index = Some(self.parts.len() - 1);
                self.open_type = Some("function_call".into());
            }
            _ => {
                self.open_index = None;
                self.open_type = None;
            }
        }
    }

    fn apply_delta(&mut self, delta: &StepDelta) {
        match delta.delta_type.as_str() {
            "text" => {
                if let Some(idx) = self.open_index
                    && let Some(part) = self.parts.get_mut(idx)
                    && let Some(chunk) = &delta.text
                {
                    match part.text.as_mut() {
                        Some(buf) => buf.push_str(chunk),
                        None => part.text = Some(chunk.clone()),
                    }
                }
            }
            "thought_summary" => {
                let chunk = delta
                    .content
                    .as_ref()
                    .and_then(|c| c.get("text"))
                    .and_then(|v| v.as_str())
                    .or(delta.text.as_deref())
                    .unwrap_or("");
                if chunk.is_empty() {
                    return;
                }
                if let Some(idx) = self.open_index
                    && let Some(part) = self.parts.get_mut(idx)
                {
                    part.thought = true;
                    match part.text.as_mut() {
                        Some(buf) => buf.push_str(chunk),
                        None => part.text = Some(chunk.to_string()),
                    }
                }
            }
            "thought_signature" => {
                if let Some(sig) = &delta.signature
                    && let Some(idx) = self.open_index
                    && let Some(part) = self.parts.get_mut(idx)
                {
                    part.thought_signature = Some(sig.clone());
                }
            }
            "arguments_delta" | "arguments" => {
                if let Some(Value::Object(map)) = &delta.arguments
                    && !map.is_empty()
                {
                    // Whole-object payload replaces any prior fragment buffer.
                    self.open_args_buf = serde_json::to_string(map).unwrap_or_default();
                } else if let Some(frag) = delta_argument_fragment(delta) {
                    self.open_args_buf.push_str(&frag);
                } else {
                    return;
                }
                if let Some(idx) = self.open_index
                    && let Some(part) = self.parts.get_mut(idx)
                    && let Some(fc) = part.function_call.as_mut()
                    && let Some(parsed) = parse_args_buffer(&self.open_args_buf)
                {
                    fc.args = Some(parsed);
                }
            }
            _ => {}
        }
    }

    fn finish_open_step(&mut self) {
        if let Some(idx) = self.open_index
            && self.open_type.as_deref() == Some("function_call")
            && let Some(part) = self.parts.get_mut(idx)
            && let Some(fc) = part.function_call.as_mut()
        {
            if let Some(parsed) = parse_args_buffer(&self.open_args_buf) {
                fc.args = Some(parsed);
            } else if args_are_empty(fc.args.as_ref()) {
                fc.args = None;
            }
            if fc.id.is_empty() && !self.open_fc_id.is_empty() {
                fc.id = self.open_fc_id.clone();
            }
            if fc.name.is_empty() {
                fc.name = self.open_fc_name.clone();
            }
        }
        self.open_index = None;
        self.open_type = None;
        self.open_args_buf.clear();
        self.open_fc_name.clear();
        self.open_fc_id.clear();
    }

    pub fn content(&self) -> Content {
        Content {
            role: Role::Model,
            parts: self.parts.clone(),
        }
    }

    pub fn into_result(self) -> InteractionResult {
        InteractionResult {
            id: self.interaction_id.unwrap_or_default(),
            content: Content {
                role: Role::Model,
                parts: self.parts,
            },
            usage: self.usage,
            status: self.status,
        }
    }
}

/// JSON response_format object for structured output.
pub fn json_response_format(schema: Value) -> Value {
    json!({
        "type": "text",
        "mime_type": "application/json",
        "schema": schema,
    })
}

fn args_are_empty(args: Option<&Value>) -> bool {
    match args {
        None => true,
        Some(Value::Null) => true,
        Some(Value::Object(map)) => map.is_empty(),
        Some(Value::String(s)) => s.trim().is_empty() || s.trim() == "{}",
        _ => false,
    }
}

/// Coerce wire args (object, or JSON-encoded string) into a usable Value.
fn normalize_function_args(args: Option<Value>) -> Option<Value> {
    match args {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed == "{}" {
                return None;
            }
            serde_json::from_str(trimmed)
                .ok()
                .or_else(|| Some(Value::String(s)))
        }
        Some(Value::Object(map)) if map.is_empty() => None,
        Some(other) => Some(other),
    }
}

fn parse_args_buffer(buf: &str) -> Option<Value> {
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn delta_argument_fragment(delta: &StepDelta) -> Option<String> {
    if let Some(partial) = delta
        .partial_arguments
        .as_ref()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
    {
        return Some(partial.to_string());
    }
    match &delta.arguments {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// True when completed-interaction steps look like a full model turn with usable tool args.
fn interaction_steps_are_authoritative(content: &Content) -> bool {
    if content.parts.is_empty() {
        return false;
    }
    let has_fc = content.parts.iter().any(|p| p.function_call.is_some());
    if !has_fc {
        return true;
    }
    // Prefer streamed accumulation when every function_call still has empty args.
    content.parts.iter().any(|p| {
        p.function_call
            .as_ref()
            .is_some_and(|fc| !args_are_empty(fc.args.as_ref()))
    }) || content
        .parts
        .iter()
        .any(|p| p.text.as_ref().is_some_and(|t| !t.is_empty()) && !p.thought)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_maps_to_metadata() {
        let usage = InteractionUsage {
            total_input_tokens: Some(10),
            total_output_tokens: Some(5),
            total_thought_tokens: Some(3),
            total_cached_tokens: Some(2),
            total_tokens: Some(18),
        };
        let meta = usage.to_usage_metadata();
        assert_eq!(meta.prompt_token_count, Some(10));
        assert_eq!(meta.candidates_token_count, Some(5));
        assert_eq!(meta.thoughts_token_count, Some(3));
        assert_eq!(meta.cached_content_token_count, Some(2));
        assert_eq!(meta.total_token_count, Some(18));
        assert_eq!(meta.context_tokens(), Some(18));
    }

    #[test]
    fn user_text_to_input() {
        let content = Content::text(Role::User, "Hello");
        let input = user_content_to_interaction_input(&content);
        assert_eq!(input["type"], "text");
        assert_eq!(input["text"], "Hello");
    }

    #[test]
    fn function_result_steps() {
        let content = Content {
            role: Role::User,
            parts: vec![Part {
                function_response: Some(FunctionResponse {
                    function_response_id: "fc_1".into(),
                    name: "get_weather".into(),
                    response: Some(json!({"temp": 52})),
                    ..Default::default()
                }),
                ..Default::default()
            }],
        };
        let steps = function_result_steps_from_content(&content).expect("steps");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0]["type"], "function_result");
        assert_eq!(steps[0]["call_id"], "fc_1");
        assert_eq!(steps[0]["name"], "get_weather");
    }

    #[test]
    fn steps_to_content_skips_echoed_user() {
        let steps = vec![
            InteractionStep {
                step_type: "user_input".into(),
                content: Some(vec![json!({"type":"text","text":"hi"})]),
                ..Default::default()
            },
            InteractionStep {
                step_type: "thought".into(),
                signature: Some("sig".into()),
                summary: Some(vec![json!({"type":"text","text":"thinking"})]),
                ..Default::default()
            },
            InteractionStep {
                step_type: "model_output".into(),
                content: Some(vec![json!({"type":"text","text":"hello"})]),
                ..Default::default()
            },
            InteractionStep {
                step_type: "function_call".into(),
                id: Some("fc_1".into()),
                name: Some("tool".into()),
                arguments: Some(json!({"a": 1})),
                ..Default::default()
            },
        ];
        let content = steps_to_model_content(&steps);
        assert_eq!(content.role, Role::Model);
        assert_eq!(content.parts.len(), 3);
        assert!(content.parts[0].thought);
        assert_eq!(content.parts[0].text.as_deref(), Some("thinking"));
        assert_eq!(content.parts[0].thought_signature.as_deref(), Some("sig"));
        assert_eq!(content.parts[1].text.as_deref(), Some("hello"));
        let fc = content.parts[2].function_call.as_ref().unwrap();
        assert_eq!(fc.id, "fc_1");
        assert_eq!(fc.name, "tool");
    }

    #[test]
    fn bootstrap_history_includes_roles() {
        let contents = vec![
            Content::text(Role::User, "Hi"),
            Content::text(Role::Model, "Hello"),
            Content::text(Role::User, "Name?"),
        ];
        let input = contents_to_interaction_input(&contents);
        let arr = input.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["type"], "user_input");
        assert_eq!(arr[1]["type"], "model_output");
        assert_eq!(arr[2]["type"], "user_input");
    }

    #[test]
    fn media_block_type_from_mime() {
        assert_eq!(media_block_type("image/png"), "image");
        assert_eq!(media_block_type("application/pdf"), "document");
        assert_eq!(media_block_type("audio/wav"), "audio");
        assert_eq!(media_block_type("video/mp4"), "video");
    }

    #[test]
    fn tool_from_declaration() {
        let decl = FunctionDeclaration {
            name: "get_weather".into(),
            description: "Weather".into(),
            parameters: Some(json!({"type":"object"})),
        };
        let tool = InteractionTool::from(&decl);
        assert_eq!(tool.tool_type, "function");
        assert_eq!(tool.name, "get_weather");
    }

    #[test]
    fn stream_accumulator_text_deltas() {
        let mut acc = StreamAccumulator::default();
        acc.on_event(&InteractionSseEvent {
            event_type: "interaction.created".into(),
            interaction: Some(Interaction {
                id: "int_1".into(),
                status: "in_progress".into(),
                ..Default::default()
            }),
            ..Default::default()
        });
        acc.on_event(&InteractionSseEvent {
            event_type: "step.start".into(),
            step: Some(InteractionStep {
                step_type: "model_output".into(),
                ..Default::default()
            }),
            ..Default::default()
        });
        acc.on_event(&InteractionSseEvent {
            event_type: "step.delta".into(),
            delta: Some(StepDelta {
                delta_type: "text".into(),
                text: Some("Hel".into()),
                ..Default::default()
            }),
            ..Default::default()
        });
        acc.on_event(&InteractionSseEvent {
            event_type: "step.delta".into(),
            delta: Some(StepDelta {
                delta_type: "text".into(),
                text: Some("lo".into()),
                ..Default::default()
            }),
            ..Default::default()
        });
        acc.on_event(&InteractionSseEvent {
            event_type: "step.stop".into(),
            ..Default::default()
        });
        assert_eq!(acc.interaction_id.as_deref(), Some("int_1"));
        assert_eq!(acc.content().parts[0].text.as_deref(), Some("Hello"));
    }

    #[test]
    fn stream_accumulator_function_args_ignore_empty_start_object() {
        // Mirrors the Interactions REST stream sample: step.start ships
        // `arguments: {}`, then arguments_delta sends the JSON string.
        let mut acc = StreamAccumulator::default();
        acc.on_event(&InteractionSseEvent {
            event_type: "step.start".into(),
            step: Some(InteractionStep {
                step_type: "function_call".into(),
                id: Some("fc_1".into()),
                name: Some("get_weather".into()),
                arguments: Some(json!({})),
                ..Default::default()
            }),
            ..Default::default()
        });
        acc.on_event(&InteractionSseEvent {
            event_type: "step.delta".into(),
            delta: Some(StepDelta {
                delta_type: "arguments_delta".into(),
                arguments: Some(Value::String(
                    r#"{"location": "Boston, MA"}"#.into(),
                )),
                ..Default::default()
            }),
            ..Default::default()
        });
        acc.on_event(&InteractionSseEvent {
            event_type: "step.stop".into(),
            ..Default::default()
        });
        // Completed event without steps must not wipe streamed args.
        acc.on_event(&InteractionSseEvent {
            event_type: "interaction.completed".into(),
            interaction: Some(Interaction {
                id: "int_1".into(),
                status: "requires_action".into(),
                usage: Some(InteractionUsage {
                    total_tokens: Some(10),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        });

        let content = acc.content();
        let fc = content.parts[0]
            .function_call
            .as_ref()
            .expect("function call");
        assert_eq!(fc.id, "fc_1");
        assert_eq!(fc.name, "get_weather");
        assert_eq!(fc.args, Some(json!({"location": "Boston, MA"})));
        assert_eq!(acc.interaction_id.as_deref(), Some("int_1"));
    }

    #[test]
    fn normalize_function_args_parses_json_string() {
        assert_eq!(
            normalize_function_args(Some(Value::String(
                r#"{"path":"/tmp"}"#.into()
            ))),
            Some(json!({"path": "/tmp"}))
        );
        assert_eq!(normalize_function_args(Some(json!({}))), None);
    }
}

impl Default for InteractionStep {
    fn default() -> Self {
        Self {
            step_type: String::new(),
            id: None,
            status: None,
            name: None,
            arguments: None,
            call_id: None,
            signature: None,
            summary: None,
            content: None,
            result: None,
            is_error: None,
        }
    }
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            id: String::new(),
            status: String::new(),
            steps: Vec::new(),
            usage: None,
            output_text: None,
            error: None,
        }
    }
}

impl Default for InteractionSseEvent {
    fn default() -> Self {
        Self {
            event_type: String::new(),
            index: None,
            interaction: None,
            interaction_id: None,
            status: None,
            step: None,
            delta: None,
        }
    }
}

impl Default for StepDelta {
    fn default() -> Self {
        Self {
            delta_type: String::new(),
            text: None,
            signature: None,
            content: None,
            arguments: None,
            partial_arguments: None,
        }
    }
}
