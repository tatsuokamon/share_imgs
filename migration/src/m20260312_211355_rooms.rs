use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Room::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Room::Id).uuid().primary_key())
                    .col(ColumnDef::new(Room::Keyword).string().not_null())
                    .col(ColumnDef::new(Room::MasterId).uuid().not_null())
                    .col(ColumnDef::new(Room::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Room::DeletedAt).timestamp())
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
pub enum Room {
    Table,

    Id,
    Keyword,
    MasterId,
    CreatedAt,
    DeletedAt,
}
