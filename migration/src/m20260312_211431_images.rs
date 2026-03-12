use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Images::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Images::Id).primary_key().uuid())
                    .col(ColumnDef::new(Images::Title).text())
                    .col(ColumnDef::new(Images::Score).integer().not_null())
                    .col(ColumnDef::new(Images::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Images::DeletedAt).timestamp())
                    .col(ColumnDef::new(Images::RoomId).uuid().not_null())
                    .col(ColumnDef::new(Images::UserId).uuid().not_null())
                    .col(ColumnDef::new(Images::ObjectKey).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-images-roomid")
                            .from(Images::Table, Images::RoomId)
                            .to(Room::Table, Room::Id)
                            .on_delete(ForeignKeyAction::NoAction),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-images-userid")
                            .from(Images::Table, Images::UserId)
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
pub enum Images {
    Table,

    Id,
    Title,
    Score,

    RoomId,
    UserId,
    ObjectKey,

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
    DeletedAt,
}

#[derive(Iden)]
pub enum User {
    Table,

    Id,
    CreatedAt,
}
