use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Comment::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Comment::Id).uuid().primary_key())
                    .col(ColumnDef::new(Comment::RoomId).uuid().not_null())
                    .col(ColumnDef::new(Comment::UserId).uuid().not_null())
                    .col(ColumnDef::new(Comment::Content).text().not_null())
                    .col(ColumnDef::new(Comment::DisplayName).text())
                    .col(ColumnDef::new(Comment::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Comment::DeletedAt).timestamp())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-comments-roomid")
                            .from(Comment::Table, Comment::RoomId)
                            .to(Room::Table, Room::Id)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-images-userid")
                            .from(Comment::Table, Comment::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

        manager
            .drop_table(Table::drop().table("post").to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Comment {
    Table,

    Id,
    RoomId,
    UserId,

    Content,
    DisplayName,
    CreatedAt,
    DeletedAt,
}

#[derive(Iden)]
pub enum Room {
    Table,

    Id,
    Keyword,
    MasterId,
    CreatedAt,
    DeleetedAt,
}

#[derive(Iden)]
pub enum User {
    Table,

    Id,
    CreatedAt,
}
