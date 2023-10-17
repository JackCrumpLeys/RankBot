use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Guilds::Table)
                    .modify_column(ColumnDef::new(Guilds::Score).float().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .modify_column(ColumnDef::new(Channels::Score).float().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .modify_column(ColumnDef::new(Messages::Score).float().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(ColumnDef::new(Users::Score).float().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Guilds::Table)
                    .modify_column(ColumnDef::new(Guilds::Score).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .modify_column(ColumnDef::new(Channels::Score).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .modify_column(ColumnDef::new(Messages::Score).integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(ColumnDef::new(Users::Score).integer().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Guilds {
    Table,
    Score,
}

#[derive(Iden)]
enum Channels {
    Table,
    Score,
}

#[derive(Iden)]
enum Users {
    Table,
    Score,
}

#[derive(Iden)]
enum Messages {
    Table,
    Score,
}
