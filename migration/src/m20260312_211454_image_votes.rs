use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ImageVote::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ImageVote::ImageId).uuid().not_null())
                    .col(ColumnDef::new(ImageVote::UserId).uuid().not_null())
                    .col(ColumnDef::new(ImageVote::IsGood).boolean().not_null())
                    .col(ColumnDef::new(ImageVote::CreatedAt).timestamp().not_null())
                    .primary_key(
                        Index::create()
                            .col(ImageVote::ImageId)
                            .col(ImageVote::UserId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-image_votes-user")
                            .from(ImageVote::Table, ImageVote::UserId)
                            .to(User::Table, User::Id).on_delete(ForeignKeyAction::NoAction),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-image_votes-image")
                            .from(ImageVote::Table, ImageVote::ImageId)
                            .to(Images::Table, Images::Id).on_delete(ForeignKeyAction::NoAction),
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
pub enum ImageVote {
    Table,
    ImageId,
    UserId,
    IsGood,
    CreatedAt,
}

#[derive(Iden)]
pub enum User {
    Table,

    Id,
    CreatedAt,
}

#[derive(Iden)]
pub enum Images {
    Table,

    Id,
    Score,
    Title,

    RoomId,
    UserId,
    ObjectKey,

    CreatedAt,
    DeletedAt,
}
