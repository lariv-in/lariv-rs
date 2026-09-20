use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MeetsConferenceRooms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MeetsConferenceRooms::Code)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(MeetsConferenceRooms::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MeetsConferenceRooms::CreatedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MeetsConferenceRooms::AnonymousAllowed)
                            .boolean()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MeetsConferenceRooms::JoiningAllowed)
                            .boolean()
                            .not_null(),
                    )
                    .col(ColumnDef::new(MeetsConferenceRooms::StartAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_meets_conference_rooms_created_by_id")
                            .from(
                                MeetsConferenceRooms::Table,
                                MeetsConferenceRooms::CreatedById,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_meets_conference_rooms_created_by_id")
                    .table(MeetsConferenceRooms::Table)
                    .col(MeetsConferenceRooms::CreatedById)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MeetsAnonymousUsers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MeetsAnonymousUsers::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MeetsAnonymousUsers::Name).text().not_null())
                    .col(ColumnDef::new(MeetsAnonymousUsers::Email).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MeetsJoinedUsers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MeetsJoinedUsers::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(MeetsJoinedUsers::ConferenceRoomCode)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MeetsJoinedUsers::UserType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(MeetsJoinedUsers::UserId).big_integer())
                    .col(ColumnDef::new(MeetsJoinedUsers::AnonymousUserId).big_integer())
                    .col(
                        ColumnDef::new(MeetsJoinedUsers::JoinedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_meets_joined_users_room_code")
                            .from(
                                MeetsJoinedUsers::Table,
                                MeetsJoinedUsers::ConferenceRoomCode,
                            )
                            .to(MeetsConferenceRooms::Table, MeetsConferenceRooms::Code)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_meets_joined_users_user_id")
                            .from(MeetsJoinedUsers::Table, MeetsJoinedUsers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_meets_joined_users_anonymous_user_id")
                            .from(MeetsJoinedUsers::Table, MeetsJoinedUsers::AnonymousUserId)
                            .to(MeetsAnonymousUsers::Table, MeetsAnonymousUsers::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_meets_joined_users_room_code")
                    .table(MeetsJoinedUsers::Table)
                    .col(MeetsJoinedUsers::ConferenceRoomCode)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MeetsMeetingRecordings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MeetsMeetingRecordings::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(MeetsMeetingRecordings::ConferenceRoomCode)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MeetsMeetingRecordings::MeetingStartAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MeetsMeetingRecordings::MeetingCreatedById)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MeetsMeetingRecordings::VideoRecordingId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MeetsMeetingRecordings::Transcript)
                            .text()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_meets_recordings_room_code")
                            .from(
                                MeetsMeetingRecordings::Table,
                                MeetsMeetingRecordings::ConferenceRoomCode,
                            )
                            .to(MeetsConferenceRooms::Table, MeetsConferenceRooms::Code)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_meets_recordings_created_by_id")
                            .from(
                                MeetsMeetingRecordings::Table,
                                MeetsMeetingRecordings::MeetingCreatedById,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_meets_recordings_video_recording_id")
                            .from(
                                MeetsMeetingRecordings::Table,
                                MeetsMeetingRecordings::VideoRecordingId,
                            )
                            .to(FilesystemNodes::Table, FilesystemNodes::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(MeetsMeetingRecordings::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(MeetsJoinedUsers::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MeetsAnonymousUsers::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(MeetsConferenceRooms::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum MeetsConferenceRooms {
    Table,
    Code,
    CreatedAt,
    CreatedById,
    AnonymousAllowed,
    JoiningAllowed,
    StartAt,
}

#[derive(Iden)]
enum MeetsAnonymousUsers {
    Table,
    Id,
    Name,
    Email,
}

#[derive(Iden)]
enum MeetsJoinedUsers {
    Table,
    Id,
    ConferenceRoomCode,
    UserType,
    UserId,
    AnonymousUserId,
    JoinedAt,
}

#[derive(Iden)]
enum MeetsMeetingRecordings {
    Table,
    Id,
    ConferenceRoomCode,
    MeetingStartAt,
    MeetingCreatedById,
    VideoRecordingId,
    Transcript,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[derive(Iden)]
enum FilesystemNodes {
    Table,
    Id,
}
