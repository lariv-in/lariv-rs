//! Tables for Go `p_llm_assistant` AutoMigrate models (sessions, parts, skills).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn gorm_model_cols<T: Iden + 'static>(
    mut table: TableCreateStatement,
    id: T,
    created_at: T,
    updated_at: T,
    deleted_at: T,
) -> TableCreateStatement {
    table
        .col(
            ColumnDef::new(id)
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        .col(ColumnDef::new(created_at).timestamp_with_time_zone())
        .col(ColumnDef::new(updated_at).timestamp_with_time_zone())
        .col(ColumnDef::new(deleted_at).timestamp_with_time_zone())
        .to_owned()
}

fn part_payload_base<T: Iden + Copy + 'static>(
    name: T,
    id: T,
    created_at: T,
    updated_at: T,
    deleted_at: T,
    part_fk: T,
) -> TableCreateStatement {
    gorm_model_cols(
        Table::create().table(name).if_not_exists().to_owned(),
        id,
        created_at,
        updated_at,
        deleted_at,
    )
    .col(ColumnDef::new(part_fk).big_integer().not_null())
    .foreign_key(
        ForeignKey::create()
            .from(name, part_fk)
            .to(MessageParts::Table, MessageParts::Id)
            .on_delete(ForeignKeyAction::Cascade),
    )
    .to_owned()
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // --- sessions ---
        manager
            .create_table(
                gorm_model_cols(
                    Table::create()
                        .table(Sessions::Table)
                        .if_not_exists()
                        .to_owned(),
                    Sessions::Id,
                    Sessions::CreatedAt,
                    Sessions::UpdatedAt,
                    Sessions::DeletedAt,
                )
                .col(
                    ColumnDef::new(Sessions::Title)
                        .text()
                        .not_null()
                        .default(""),
                )
                .col(ColumnDef::new(Sessions::UserId).big_integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_llm_assistant_sessions_user_id")
                        .from(Sessions::Table, Sessions::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_assistant_sessions_user_id")
                    .table(Sessions::Table)
                    .col(Sessions::UserId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_assistant_sessions_deleted_at")
                    .table(Sessions::Table)
                    .col(Sessions::DeletedAt)
                    .to_owned(),
            )
            .await?;

        // --- messages ---
        manager
            .create_table(
                gorm_model_cols(
                    Table::create()
                        .table(Messages::Table)
                        .if_not_exists()
                        .to_owned(),
                    Messages::Id,
                    Messages::CreatedAt,
                    Messages::UpdatedAt,
                    Messages::DeletedAt,
                )
                .col(
                    ColumnDef::new(Messages::LlmAssistantSessionId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Messages::Role)
                        .text()
                        .not_null()
                        .default("user"),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_llm_assistant_session_messages_session_id")
                        .from(Messages::Table, Messages::LlmAssistantSessionId)
                        .to(Sessions::Table, Sessions::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
            )
            .await?;

        // --- video_metadata (before parts FK) ---
        manager
            .create_table(
                gorm_model_cols(
                    Table::create()
                        .table(VideoMetadata::Table)
                        .if_not_exists()
                        .to_owned(),
                    VideoMetadata::Id,
                    VideoMetadata::CreatedAt,
                    VideoMetadata::UpdatedAt,
                    VideoMetadata::DeletedAt,
                )
                .col(
                    ColumnDef::new(VideoMetadata::EndOffset)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .col(ColumnDef::new(VideoMetadata::Fps).double())
                .col(
                    ColumnDef::new(VideoMetadata::StartOffset)
                        .big_integer()
                        .not_null()
                        .default(0),
                )
                .to_owned(),
            )
            .await?;

        // --- message parts ---
        manager
            .create_table(
                gorm_model_cols(
                    Table::create()
                        .table(MessageParts::Table)
                        .if_not_exists()
                        .to_owned(),
                    MessageParts::Id,
                    MessageParts::CreatedAt,
                    MessageParts::UpdatedAt,
                    MessageParts::DeletedAt,
                )
                .col(ColumnDef::new(MessageParts::Kind).text().not_null())
                .col(
                    ColumnDef::new(MessageParts::LlmAssistantSessionMessageId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MessageParts::Thought)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(ColumnDef::new(MessageParts::ThoughtSignature).blob())
                .col(ColumnDef::new(MessageParts::VideoMetadataId).big_integer())
                .col(ColumnDef::new(MessageParts::PartMetadata).json_binary())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_llm_assistant_session_message_parts_message_id")
                        .from(
                            MessageParts::Table,
                            MessageParts::LlmAssistantSessionMessageId,
                        )
                        .to(Messages::Table, Messages::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_llm_assistant_session_message_parts_video_metadata_id")
                        .from(MessageParts::Table, MessageParts::VideoMetadataId)
                        .to(VideoMetadata::Table, VideoMetadata::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
            )
            .await?;

        // --- part payload tables ---
        manager
            .create_table(
                part_payload_base(
                    InlineData::Table,
                    InlineData::Id,
                    InlineData::CreatedAt,
                    InlineData::UpdatedAt,
                    InlineData::DeletedAt,
                    InlineData::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(InlineData::MimeType).text().not_null())
                .col(ColumnDef::new(InlineData::Data).blob().not_null())
                .col(ColumnDef::new(InlineData::DisplayName).text())
                .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    Texts::Table,
                    Texts::Id,
                    Texts::CreatedAt,
                    Texts::UpdatedAt,
                    Texts::DeletedAt,
                    Texts::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(Texts::Text).text().not_null())
                .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    FileData::Table,
                    FileData::Id,
                    FileData::CreatedAt,
                    FileData::UpdatedAt,
                    FileData::DeletedAt,
                    FileData::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(FileData::DisplayName).text())
                .col(ColumnDef::new(FileData::FileUri).text().not_null())
                .col(ColumnDef::new(FileData::MimeType).text().not_null())
                .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    FunctionCalls::Table,
                    FunctionCalls::Id,
                    FunctionCalls::CreatedAt,
                    FunctionCalls::UpdatedAt,
                    FunctionCalls::DeletedAt,
                    FunctionCalls::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(FunctionCalls::FunctionCallId).text())
                .col(ColumnDef::new(FunctionCalls::Args).json_binary())
                .col(ColumnDef::new(FunctionCalls::Name).text())
                .col(ColumnDef::new(FunctionCalls::WillContinue).boolean())
                .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_asst_msg_function_calls_fc_id")
                    .table(FunctionCalls::Table)
                    .col(FunctionCalls::FunctionCallId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    FunctionResponses::Table,
                    FunctionResponses::Id,
                    FunctionResponses::CreatedAt,
                    FunctionResponses::UpdatedAt,
                    FunctionResponses::DeletedAt,
                    FunctionResponses::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(FunctionResponses::WillContinue).boolean())
                .col(
                    ColumnDef::new(FunctionResponses::Scheduling)
                        .text()
                        .default("WHEN_IDLE"),
                )
                .col(ColumnDef::new(FunctionResponses::FunctionResponseId).text())
                .col(ColumnDef::new(FunctionResponses::Name).text().not_null())
                .col(ColumnDef::new(FunctionResponses::Response).json_binary())
                .to_owned(),
            )
            .await?;

        manager
            .create_table(
                gorm_model_cols(
                    Table::create()
                        .table(FrParts::Table)
                        .if_not_exists()
                        .to_owned(),
                    FrParts::Id,
                    FrParts::CreatedAt,
                    FrParts::UpdatedAt,
                    FrParts::DeletedAt,
                )
                .col(
                    ColumnDef::new(FrParts::LlmAssistantSessionMessageFunctionResponseId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(FrParts::Kind).text().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_llm_asst_fr_parts_fr_id")
                        .from(
                            FrParts::Table,
                            FrParts::LlmAssistantSessionMessageFunctionResponseId,
                        )
                        .to(FunctionResponses::Table, FunctionResponses::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
            )
            .await?;

        manager
            .create_table(
                gorm_model_cols(
                    Table::create()
                        .table(FrBlobs::Table)
                        .if_not_exists()
                        .to_owned(),
                    FrBlobs::Id,
                    FrBlobs::CreatedAt,
                    FrBlobs::UpdatedAt,
                    FrBlobs::DeletedAt,
                )
                .col(
                    ColumnDef::new(FrBlobs::LlmAssistantSessionMessageFunctionResponsePartId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(FrBlobs::MimeType).text().not_null())
                .col(ColumnDef::new(FrBlobs::Data).blob().not_null())
                .col(ColumnDef::new(FrBlobs::DisplayName).text())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_llm_asst_fr_blobs_part_id")
                        .from(
                            FrBlobs::Table,
                            FrBlobs::LlmAssistantSessionMessageFunctionResponsePartId,
                        )
                        .to(FrParts::Table, FrParts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
            )
            .await?;

        manager
            .create_table(
                gorm_model_cols(
                    Table::create()
                        .table(FrFileData::Table)
                        .if_not_exists()
                        .to_owned(),
                    FrFileData::Id,
                    FrFileData::CreatedAt,
                    FrFileData::UpdatedAt,
                    FrFileData::DeletedAt,
                )
                .col(
                    ColumnDef::new(FrFileData::LlmAssistantSessionMessageFunctionResponsePartId)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(FrFileData::FileUri).text().not_null())
                .col(ColumnDef::new(FrFileData::MimeType).text().not_null())
                .col(ColumnDef::new(FrFileData::DisplayName).text())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_llm_asst_fr_file_data_part_id")
                        .from(
                            FrFileData::Table,
                            FrFileData::LlmAssistantSessionMessageFunctionResponsePartId,
                        )
                        .to(FrParts::Table, FrParts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    ExecutableCodes::Table,
                    ExecutableCodes::Id,
                    ExecutableCodes::CreatedAt,
                    ExecutableCodes::UpdatedAt,
                    ExecutableCodes::DeletedAt,
                    ExecutableCodes::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(ExecutableCodes::Code).text())
                .col(ColumnDef::new(ExecutableCodes::Language).text())
                .col(ColumnDef::new(ExecutableCodes::ExecutableCodeId).text())
                .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_asst_executable_codes_id")
                    .table(ExecutableCodes::Table)
                    .col(ExecutableCodes::ExecutableCodeId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    CodeExecResults::Table,
                    CodeExecResults::Id,
                    CodeExecResults::CreatedAt,
                    CodeExecResults::UpdatedAt,
                    CodeExecResults::DeletedAt,
                    CodeExecResults::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(CodeExecResults::Outcome).text().not_null())
                .col(ColumnDef::new(CodeExecResults::Output).text())
                .col(ColumnDef::new(CodeExecResults::ExecutableCodeId).text())
                .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    ToolCalls::Table,
                    ToolCalls::Id,
                    ToolCalls::CreatedAt,
                    ToolCalls::UpdatedAt,
                    ToolCalls::DeletedAt,
                    ToolCalls::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(ToolCalls::ToolCallId).text())
                .col(ColumnDef::new(ToolCalls::ToolType).text())
                .col(ColumnDef::new(ToolCalls::Args).json_binary())
                .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_asst_tool_calls_id")
                    .table(ToolCalls::Table)
                    .col(ToolCalls::ToolCallId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    ToolResponses::Table,
                    ToolResponses::Id,
                    ToolResponses::CreatedAt,
                    ToolResponses::UpdatedAt,
                    ToolResponses::DeletedAt,
                    ToolResponses::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(ToolResponses::ToolCallId).text())
                .col(ColumnDef::new(ToolResponses::ToolType).text())
                .col(ColumnDef::new(ToolResponses::Response).json_binary())
                .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_llm_asst_tool_responses_id")
                    .table(ToolResponses::Table)
                    .col(ToolResponses::ToolCallId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                part_payload_base(
                    MediaResolutions::Table,
                    MediaResolutions::Id,
                    MediaResolutions::CreatedAt,
                    MediaResolutions::UpdatedAt,
                    MediaResolutions::DeletedAt,
                    MediaResolutions::LlmAssistantSessionMessagePartId,
                )
                .col(ColumnDef::new(MediaResolutions::Level).text().not_null())
                .col(ColumnDef::new(MediaResolutions::NumTokens).integer())
                .to_owned(),
            )
            .await?;

        // --- skills ---
        manager
            .create_table(
                gorm_model_cols(
                    Table::create()
                        .table(Skills::Table)
                        .if_not_exists()
                        .to_owned(),
                    Skills::Id,
                    Skills::CreatedAt,
                    Skills::UpdatedAt,
                    Skills::DeletedAt,
                )
                .col(
                    ColumnDef::new(Skills::Name)
                        .text()
                        .not_null()
                        .default(""),
                )
                .col(
                    ColumnDef::new(Skills::Description)
                        .text()
                        .not_null()
                        .default(""),
                )
                .col(
                    ColumnDef::new(Skills::Content)
                        .text()
                        .not_null()
                        .default(""),
                )
                .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_skills_name")
                    .table(Skills::Table)
                    .col(Skills::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_skills_deleted_at")
                    .table(Skills::Table)
                    .col(Skills::DeletedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SkillFiles::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SkillFiles::SkillId).big_integer().not_null())
                    .col(ColumnDef::new(SkillFiles::VNodeId).big_integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(SkillFiles::SkillId)
                            .col(SkillFiles::VNodeId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_skill_files_skill_id")
                            .from(SkillFiles::Table, SkillFiles::SkillId)
                            .to(Skills::Table, Skills::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_llm_assistant_skill_files_v_node_id")
                            .from(SkillFiles::Table, SkillFiles::VNodeId)
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SkillFiles::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Skills::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MediaResolutions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ToolResponses::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ToolCalls::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CodeExecResults::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ExecutableCodes::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(FrFileData::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(FrBlobs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(FrParts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(FunctionResponses::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(FunctionCalls::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(FileData::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Texts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(InlineData::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MessageParts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(VideoMetadata::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden, Copy, Clone)]
enum Users {
    #[iden = "users"]
    Table,
    Id,
}

#[derive(Iden, Copy, Clone)]
enum FilesystemNodes {
    #[iden = "filesystem_nodes"]
    Table,
    Id,
}

#[derive(Iden, Copy, Clone)]
enum Sessions {
    #[iden = "llm_assistant_sessions"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Title,
    UserId,
}

#[derive(Iden, Copy, Clone)]
enum Messages {
    #[iden = "llm_assistant_session_messages"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionId,
    Role,
}

#[derive(Iden, Copy, Clone)]
enum VideoMetadata {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    EndOffset,
    Fps,
    StartOffset,
}

#[derive(Iden, Copy, Clone)]
enum MessageParts {
    #[iden = "llm_assistant_session_message_parts"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Kind,
    LlmAssistantSessionMessageId,
    Thought,
    ThoughtSignature,
    VideoMetadataId,
    PartMetadata,
}

#[derive(Iden, Copy, Clone)]
enum InlineData {
    #[iden = "llm_assistant_session_message_inline_data"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    MimeType,
    Data,
    DisplayName,
}

#[derive(Iden, Copy, Clone)]
enum Texts {
    #[iden = "llm_assistant_session_message_texts"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    Text,
}

#[derive(Iden, Copy, Clone)]
enum FileData {
    #[iden = "llm_assistant_session_message_file_data"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    DisplayName,
    FileUri,
    MimeType,
}

#[derive(Iden, Copy, Clone)]
enum FunctionCalls {
    #[iden = "llm_assistant_session_message_function_calls"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    FunctionCallId,
    Args,
    Name,
    WillContinue,
}

#[derive(Iden, Copy, Clone)]
enum FunctionResponses {
    #[iden = "llm_assistant_session_message_function_responses"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    WillContinue,
    Scheduling,
    FunctionResponseId,
    Name,
    Response,
}

#[derive(Iden, Copy, Clone)]
enum FrParts {
    #[iden = "llm_assistant_session_message_function_response_parts"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessageFunctionResponseId,
    Kind,
}

#[derive(Iden, Copy, Clone)]
enum FrBlobs {
    #[iden = "llm_assistant_session_message_function_response_blobs"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessageFunctionResponsePartId,
    MimeType,
    Data,
    DisplayName,
}

#[derive(Iden, Copy, Clone)]
enum FrFileData {
    #[iden = "llm_assistant_session_message_function_response_file_data"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessageFunctionResponsePartId,
    FileUri,
    MimeType,
    DisplayName,
}

#[derive(Iden, Copy, Clone)]
enum ExecutableCodes {
    #[iden = "llm_assistant_session_executable_codes"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    Code,
    Language,
    ExecutableCodeId,
}

#[derive(Iden, Copy, Clone)]
enum CodeExecResults {
    #[iden = "llm_assistant_session_message_code_execution_results"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    Outcome,
    Output,
    ExecutableCodeId,
}

#[derive(Iden, Copy, Clone)]
enum ToolCalls {
    #[iden = "llm_assistant_session_message_tool_calls"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    ToolCallId,
    ToolType,
    Args,
}

#[derive(Iden, Copy, Clone)]
enum ToolResponses {
    #[iden = "llm_assistant_session_message_tool_responses"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    ToolCallId,
    ToolType,
    Response,
}

#[derive(Iden, Copy, Clone)]
enum MediaResolutions {
    #[iden = "llm_assistant_session_message_media_resolutions"]
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    LlmAssistantSessionMessagePartId,
    Level,
    NumTokens,
}

#[derive(Iden, Copy, Clone)]
enum Skills {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    Name,
    Description,
    Content,
}

#[derive(Iden, Copy, Clone)]
enum SkillFiles {
    #[iden = "llm_assistant_skill_files"]
    Table,
    SkillId,
    VNodeId,
}
