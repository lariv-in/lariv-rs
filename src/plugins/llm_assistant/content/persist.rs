//! SeaORM save/load for Content ↔ session messages / parts.

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder,
};
use serde_json::Value;
use thiserror::Error;

use super::kinds::{
    KIND_CODE_EXECUTION_RESULT, KIND_EXECUTABLE_CODE, KIND_FILE_DATA, KIND_FUNCTION_CALL,
    KIND_FUNCTION_RESPONSE, KIND_INLINE_DATA, KIND_MEDIA_RESOLUTION, KIND_TEXT, KIND_TOOL_CALL,
    KIND_TOOL_RESPONSE, classify_part_kind,
};
use super::sanitize::{
    genai_part_passes_chat_validate_content, sanitize_content_parts_for_genai_chat, ZWSP,
};
use crate::plugins::llm_assistant::{
    entities::{
        part_code_execution_result, part_executable_code, part_file_data, part_fr_blob,
        part_fr_file_data, part_fr_part, part_function_call, part_function_response,
        part_inline_data, part_media_resolution, part_text, part_tool_call, part_tool_response,
        session_message, session_message_part, video_metadata,
    },
    genai::{
        Blob, CodeExecutionResult, Content, ExecutableCode, FileData, FunctionCall,
        FunctionResponse, FunctionResponseBlob, FunctionResponseFileData, FunctionResponsePart,
        Part, PartMediaResolution, ToolCall, ToolResponse, VideoMetadata,
    },
};

#[derive(Debug, Error)]
pub enum PersistError {
    #[error("database: {0}")]
    Db(#[from] DbErr),
    #[error("unknown part kind: {0}")]
    UnknownKind(String),
    #[error("{0}")]
    Other(String),
}

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

fn decode_b64(s: &str) -> Vec<u8> {
    B64.decode(s.trim()).unwrap_or_else(|_| s.as_bytes().to_vec())
}

fn encode_b64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

fn duration_value_to_ns(v: &Option<Value>) -> i64 {
    match v {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => {
            // Go stores time.Duration as ns; wire may be "1.5s" — keep simple parse
            if let Ok(ns) = s.parse::<i64>() {
                return ns;
            }
            if let Some(stripped) = s.strip_suffix('s')
                && let Ok(secs) = stripped.parse::<f64>()
            {
                return (secs * 1_000_000_000.0) as i64;
            }
            0
        }
        _ => 0,
    }
}

fn ns_to_value(ns: i64) -> Value {
    Value::Number(ns.into())
}

pub async fn save_content(
    db: &DatabaseConnection,
    session_id: i64,
    content: &Content,
) -> Result<i64, PersistError> {
    let ts = now();
    let msg = session_message::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(ts)),
        updated_at: Set(Some(ts)),
        deleted_at: Set(None),
        llm_assistant_session_id: Set(session_id),
        role: Set(if content.role.is_empty() {
            "user".into()
        } else {
            content.role.clone()
        }),
    };
    let saved = msg.insert(db).await?;
    save_parts(db, saved.id, &content.parts).await?;
    Ok(saved.id)
}

async fn save_parts(
    db: &DatabaseConnection,
    message_id: i64,
    parts: &[Part],
) -> Result<(), PersistError> {
    for part in parts {
        let kind = classify_part_kind(part)
            .ok_or_else(|| PersistError::UnknownKind(format!("{part:?}")))?;

        let mut video_metadata_id: Option<i64> = None;
        if let Some(vm) = &part.video_metadata {
            let ts = now();
            let row = video_metadata::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                end_offset: Set(duration_value_to_ns(&vm.end_offset)),
                fps: Set(vm.fps),
                start_offset: Set(duration_value_to_ns(&vm.start_offset)),
            };
            let saved = row.insert(db).await?;
            video_metadata_id = Some(saved.id);
        }

        let thought_sig = part
            .thought_signature
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| decode_b64(s));

        let part_meta = part.part_metadata.clone();

        let ts = now();
        let part_row = session_message_part::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(ts)),
            updated_at: Set(Some(ts)),
            deleted_at: Set(None),
            kind: Set(kind.to_string()),
            llm_assistant_session_message_id: Set(message_id),
            thought: Set(part.thought),
            thought_signature: Set(thought_sig),
            video_metadata_id: Set(video_metadata_id),
            part_metadata: Set(part_meta),
        };
        let saved_part = part_row.insert(db).await?;
        save_part_payload(db, saved_part.id, kind, part).await?;
    }
    Ok(())
}

async fn save_part_payload(
    db: &DatabaseConnection,
    part_id: i64,
    kind: &str,
    part: &Part,
) -> Result<(), PersistError> {
    let ts = now();
    match kind {
        KIND_TEXT => {
            part_text::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                text: Set(part.text.clone().unwrap_or_default()),
            }
            .insert(db)
            .await?;
        }
        KIND_INLINE_DATA => {
            let blob = part
                .inline_data
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing inlineData".into()))?;
            part_inline_data::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                mime_type: Set(blob.mime_type.clone()),
                data: Set(decode_b64(&blob.data)),
                display_name: Set(if blob.display_name.is_empty() {
                    None
                } else {
                    Some(blob.display_name.clone())
                }),
            }
            .insert(db)
            .await?;
        }
        KIND_FILE_DATA => {
            let fd = part
                .file_data
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing fileData".into()))?;
            part_file_data::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                display_name: Set(if fd.display_name.is_empty() {
                    None
                } else {
                    Some(fd.display_name.clone())
                }),
                file_uri: Set(fd.file_uri.clone()),
                mime_type: Set(fd.mime_type.clone()),
            }
            .insert(db)
            .await?;
        }
        KIND_FUNCTION_CALL => {
            let fc = part
                .function_call
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing functionCall".into()))?;
            part_function_call::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                function_call_id: Set(if fc.id.is_empty() {
                    None
                } else {
                    Some(fc.id.clone())
                }),
                args: Set(fc.args.clone()),
                name: Set(Some(fc.name.clone())),
                will_continue: Set(fc.will_continue),
            }
            .insert(db)
            .await?;
        }
        KIND_FUNCTION_RESPONSE => {
            let fr = part
                .function_response
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing functionResponse".into()))?;
            let scheduling = if fr.scheduling.is_empty() {
                "WHEN_IDLE".to_string()
            } else {
                fr.scheduling.clone()
            };
            let saved = part_function_response::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                will_continue: Set(fr.will_continue),
                scheduling: Set(Some(scheduling)),
                function_response_id: Set(if fr.function_response_id.is_empty() {
                    None
                } else {
                    Some(fr.function_response_id.clone())
                }),
                name: Set(fr.name.clone()),
                response: Set(fr.response.clone()),
            }
            .insert(db)
            .await?;
            for frp in &fr.parts {
                save_fr_part(db, saved.id, frp).await?;
            }
        }
        KIND_EXECUTABLE_CODE => {
            let ec = part
                .executable_code
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing executableCode".into()))?;
            part_executable_code::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                code: Set(Some(ec.code.clone())),
                language: Set(Some(ec.language.clone())),
                executable_code_id: Set(if ec.executable_code_id.is_empty() {
                    None
                } else {
                    Some(ec.executable_code_id.clone())
                }),
            }
            .insert(db)
            .await?;
        }
        KIND_CODE_EXECUTION_RESULT => {
            let cer = part
                .code_execution_result
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing codeExecutionResult".into()))?;
            part_code_execution_result::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                outcome: Set(cer.outcome.clone()),
                output: Set(Some(cer.output.clone())),
                executable_code_id: Set(if cer.executable_code_id.is_empty() {
                    None
                } else {
                    Some(cer.executable_code_id.clone())
                }),
            }
            .insert(db)
            .await?;
        }
        KIND_TOOL_CALL => {
            let tc = part
                .tool_call
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing toolCall".into()))?;
            part_tool_call::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                tool_call_id: Set(if tc.tool_call_id.is_empty() {
                    None
                } else {
                    Some(tc.tool_call_id.clone())
                }),
                tool_type: Set(if tc.tool_type.is_empty() {
                    None
                } else {
                    Some(tc.tool_type.clone())
                }),
                args: Set(tc.args.clone()),
            }
            .insert(db)
            .await?;
        }
        KIND_TOOL_RESPONSE => {
            let tr = part
                .tool_response
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing toolResponse".into()))?;
            part_tool_response::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                tool_call_id: Set(if tr.tool_call_id.is_empty() {
                    None
                } else {
                    Some(tr.tool_call_id.clone())
                }),
                tool_type: Set(if tr.tool_type.is_empty() {
                    None
                } else {
                    Some(tr.tool_type.clone())
                }),
                response: Set(tr.response.clone()),
            }
            .insert(db)
            .await?;
        }
        KIND_MEDIA_RESOLUTION => {
            let mr = part
                .media_resolution
                .as_ref()
                .ok_or_else(|| PersistError::Other("missing mediaResolution".into()))?;
            part_media_resolution::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_part_id: Set(part_id),
                level: Set(mr.level.clone()),
                num_tokens: Set(mr.num_tokens),
            }
            .insert(db)
            .await?;
        }
        other => return Err(PersistError::UnknownKind(other.into())),
    }
    Ok(())
}

async fn save_fr_part(
    db: &DatabaseConnection,
    fr_id: i64,
    frp: &FunctionResponsePart,
) -> Result<(), PersistError> {
    let kind = if frp.inline_data.is_some() {
        KIND_INLINE_DATA
    } else if frp.file_data.is_some() {
        KIND_FILE_DATA
    } else {
        return Err(PersistError::Other(
            "unknown function response part kind".into(),
        ));
    };
    let ts = now();
    let saved = part_fr_part::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(ts)),
        updated_at: Set(Some(ts)),
        deleted_at: Set(None),
        llm_assistant_session_message_function_response_id: Set(fr_id),
        kind: Set(kind.to_string()),
    }
    .insert(db)
    .await?;

    match kind {
        KIND_INLINE_DATA => {
            let blob = frp.inline_data.as_ref().unwrap();
            part_fr_blob::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_function_response_part_id: Set(saved.id),
                mime_type: Set(blob.mime_type.clone()),
                data: Set(decode_b64(&blob.data)),
                display_name: Set(if blob.display_name.is_empty() {
                    None
                } else {
                    Some(blob.display_name.clone())
                }),
            }
            .insert(db)
            .await?;
        }
        KIND_FILE_DATA => {
            let fd = frp.file_data.as_ref().unwrap();
            part_fr_file_data::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(ts)),
                updated_at: Set(Some(ts)),
                deleted_at: Set(None),
                llm_assistant_session_message_function_response_part_id: Set(saved.id),
                file_uri: Set(fd.file_uri.clone()),
                mime_type: Set(fd.mime_type.clone()),
                display_name: Set(if fd.display_name.is_empty() {
                    None
                } else {
                    Some(fd.display_name.clone())
                }),
            }
            .insert(db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn apply_part_common(
    db: &DatabaseConnection,
    row: &session_message_part::Model,
    mut part: Part,
) -> Result<Part, PersistError> {
    part.thought = row.thought;
    if let Some(sig) = &row.thought_signature
        && !sig.is_empty()
    {
        part.thought_signature = Some(encode_b64(sig));
    }
    if let Some(vmid) = row.video_metadata_id
        && let Some(vm) = video_metadata::Entity::find_by_id(vmid).one(db).await?
    {
        part.video_metadata = Some(VideoMetadata {
            end_offset: Some(ns_to_value(vm.end_offset)),
            fps: vm.fps,
            start_offset: Some(ns_to_value(vm.start_offset)),
        });
    }
    if let Some(meta) = &row.part_metadata {
        part.part_metadata = Some(meta.clone());
    }
    Ok(part)
}

async fn load_part(
    db: &DatabaseConnection,
    row: &session_message_part::Model,
) -> Result<Part, PersistError> {
    let part_id = row.id;
    let part = match row.kind.as_str() {
        KIND_TEXT => {
            let text = part_text::Entity::find()
                .filter(part_text::Column::LlmAssistantSessionMessagePartId.eq(part_id))
                .one(db)
                .await?
                .map(|t| t.text)
                .unwrap_or_default();
            let mut p = Part {
                text: Some(text),
                ..Default::default()
            };
            p = apply_part_common(db, row, p).await?;
            if !genai_part_passes_chat_validate_content(&p) {
                p.text = Some(ZWSP.to_string());
            }
            p
        }
        KIND_INLINE_DATA => {
            let blob = part_inline_data::Entity::find()
                .filter(part_inline_data::Column::LlmAssistantSessionMessagePartId.eq(part_id))
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("inlineData row missing".into()))?;
            apply_part_common(
                db,
                row,
                Part {
                    inline_data: Some(Blob {
                        mime_type: blob.mime_type,
                        data: encode_b64(&blob.data),
                        display_name: blob.display_name.unwrap_or_default(),
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        KIND_FILE_DATA => {
            let fd = part_file_data::Entity::find()
                .filter(part_file_data::Column::LlmAssistantSessionMessagePartId.eq(part_id))
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("fileData row missing".into()))?;
            apply_part_common(
                db,
                row,
                Part {
                    file_data: Some(FileData {
                        display_name: fd.display_name.unwrap_or_default(),
                        file_uri: fd.file_uri,
                        mime_type: fd.mime_type,
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        KIND_FUNCTION_CALL => {
            let fc = part_function_call::Entity::find()
                .filter(part_function_call::Column::LlmAssistantSessionMessagePartId.eq(part_id))
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("functionCall row missing".into()))?;
            apply_part_common(
                db,
                row,
                Part {
                    function_call: Some(FunctionCall {
                        id: fc.function_call_id.unwrap_or_default(),
                        name: fc.name.unwrap_or_default(),
                        args: fc.args,
                        will_continue: fc.will_continue,
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        KIND_FUNCTION_RESPONSE => {
            let fr = part_function_response::Entity::find()
                .filter(
                    part_function_response::Column::LlmAssistantSessionMessagePartId.eq(part_id),
                )
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("functionResponse row missing".into()))?;
            let fr_parts_rows = part_fr_part::Entity::find()
                .filter(
                    part_fr_part::Column::LlmAssistantSessionMessageFunctionResponseId.eq(fr.id),
                )
                .all(db)
                .await?;
            let mut fr_parts = Vec::new();
            for frp in fr_parts_rows {
                fr_parts.push(load_fr_part(db, &frp).await?);
            }
            apply_part_common(
                db,
                row,
                Part {
                    function_response: Some(FunctionResponse {
                        will_continue: fr.will_continue,
                        scheduling: fr.scheduling.unwrap_or_else(|| "WHEN_IDLE".into()),
                        function_response_id: fr.function_response_id.unwrap_or_default(),
                        name: fr.name,
                        response: fr.response,
                        parts: fr_parts,
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        KIND_EXECUTABLE_CODE => {
            let ec = part_executable_code::Entity::find()
                .filter(part_executable_code::Column::LlmAssistantSessionMessagePartId.eq(part_id))
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("executableCode row missing".into()))?;
            apply_part_common(
                db,
                row,
                Part {
                    executable_code: Some(ExecutableCode {
                        code: ec.code.unwrap_or_default(),
                        language: ec.language.unwrap_or_default(),
                        executable_code_id: ec.executable_code_id.unwrap_or_default(),
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        KIND_CODE_EXECUTION_RESULT => {
            let cer = part_code_execution_result::Entity::find()
                .filter(
                    part_code_execution_result::Column::LlmAssistantSessionMessagePartId
                        .eq(part_id),
                )
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("codeExecutionResult row missing".into()))?;
            apply_part_common(
                db,
                row,
                Part {
                    code_execution_result: Some(CodeExecutionResult {
                        outcome: cer.outcome,
                        output: cer.output.unwrap_or_default(),
                        executable_code_id: cer.executable_code_id.unwrap_or_default(),
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        KIND_TOOL_CALL => {
            let tc = part_tool_call::Entity::find()
                .filter(part_tool_call::Column::LlmAssistantSessionMessagePartId.eq(part_id))
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("toolCall row missing".into()))?;
            apply_part_common(
                db,
                row,
                Part {
                    tool_call: Some(ToolCall {
                        tool_call_id: tc.tool_call_id.unwrap_or_default(),
                        tool_type: tc.tool_type.unwrap_or_default(),
                        args: tc.args,
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        KIND_TOOL_RESPONSE => {
            let tr = part_tool_response::Entity::find()
                .filter(part_tool_response::Column::LlmAssistantSessionMessagePartId.eq(part_id))
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("toolResponse row missing".into()))?;
            apply_part_common(
                db,
                row,
                Part {
                    tool_response: Some(ToolResponse {
                        tool_call_id: tr.tool_call_id.unwrap_or_default(),
                        tool_type: tr.tool_type.unwrap_or_default(),
                        response: tr.response,
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        KIND_MEDIA_RESOLUTION => {
            let mr = part_media_resolution::Entity::find()
                .filter(
                    part_media_resolution::Column::LlmAssistantSessionMessagePartId.eq(part_id),
                )
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("mediaResolution row missing".into()))?;
            apply_part_common(
                db,
                row,
                Part {
                    media_resolution: Some(PartMediaResolution {
                        level: mr.level,
                        num_tokens: mr.num_tokens,
                    }),
                    ..Default::default()
                },
            )
            .await?
        }
        other => return Err(PersistError::UnknownKind(other.into())),
    };
    Ok(part)
}

async fn load_fr_part(
    db: &DatabaseConnection,
    row: &part_fr_part::Model,
) -> Result<FunctionResponsePart, PersistError> {
    match row.kind.as_str() {
        KIND_INLINE_DATA => {
            let blob = part_fr_blob::Entity::find()
                .filter(
                    part_fr_blob::Column::LlmAssistantSessionMessageFunctionResponsePartId
                        .eq(row.id),
                )
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("FR blob missing".into()))?;
            Ok(FunctionResponsePart {
                inline_data: Some(FunctionResponseBlob {
                    mime_type: blob.mime_type,
                    data: encode_b64(&blob.data),
                    display_name: blob.display_name.unwrap_or_default(),
                }),
                file_data: None,
            })
        }
        KIND_FILE_DATA => {
            let fd = part_fr_file_data::Entity::find()
                .filter(
                    part_fr_file_data::Column::LlmAssistantSessionMessageFunctionResponsePartId
                        .eq(row.id),
                )
                .one(db)
                .await?
                .ok_or_else(|| PersistError::Other("FR fileData missing".into()))?;
            Ok(FunctionResponsePart {
                inline_data: None,
                file_data: Some(FunctionResponseFileData {
                    file_uri: fd.file_uri,
                    mime_type: fd.mime_type,
                    display_name: fd.display_name.unwrap_or_default(),
                }),
            })
        }
        other => Err(PersistError::UnknownKind(other.into())),
    }
}

pub async fn load_content(
    db: &DatabaseConnection,
    message: &session_message::Model,
) -> Result<Content, PersistError> {
    let parts = session_message_part::Entity::find()
        .filter(session_message_part::Column::LlmAssistantSessionMessageId.eq(message.id))
        .filter(session_message_part::Column::DeletedAt.is_null())
        .order_by_asc(session_message_part::Column::Id)
        .all(db)
        .await?;
    let mut out_parts = Vec::with_capacity(parts.len());
    for row in &parts {
        out_parts.push(load_part(db, row).await?);
    }
    Ok(Content {
        role: message.role.clone(),
        parts: out_parts,
    })
}

pub async fn load_session_contents(
    db: &DatabaseConnection,
    session_id: i64,
) -> Result<Vec<Content>, PersistError> {
    let messages = session_message::Entity::find()
        .filter(session_message::Column::LlmAssistantSessionId.eq(session_id))
        .filter(session_message::Column::DeletedAt.is_null())
        .order_by_asc(session_message::Column::Id)
        .all(db)
        .await?;
    let mut contents = Vec::with_capacity(messages.len());
    for message in &messages {
        let mut content = load_content(db, message).await?;
        sanitize_content_parts_for_genai_chat(&mut content);
        contents.push(content);
    }
    Ok(contents)
}
