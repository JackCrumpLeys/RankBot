use sea_orm_migration::prelude::*;
use crate::m20231013_004433_snowflake_primary::Messages::Channel;
use crate::sea_orm::{DbBackend, Statement, StatementBuilder};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .drop_foreign_key(Alias::new("messages_messages"))
                    .drop_column(Messages::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .modify_column(
                        ColumnDef::new(Messages::Snowflake)
                            .primary_key()
                            .big_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .drop_foreign_key(Alias::new("messages_channels"))
                    .drop_foreign_key(Alias::new("messages_users"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .drop_column(Channels::Id)
                    .drop_foreign_key(Alias::new("channels_guilds"))
                    .modify_column(
                        ColumnDef::new(Channels::Snowflake)
                            .primary_key()
                            .big_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_foreign_key(Alias::new("users_guilds"))
                    .drop_column(Users::Id)
                    .modify_column(
                        ColumnDef::new(Users::Snowflake)
                            .primary_key()
                            .big_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .modify_column(ColumnDef::new(Messages::ReplysTo).big_unsigned().null())
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("FK_messages_replys_to")
                            .to(Messages::Table, Messages::Snowflake)
                            .from(Messages::Table, Messages::ReplysTo)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .modify_column(ColumnDef::new(Messages::Channel).big_unsigned().not_null())
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("FK_messages_channels")
                            .to(Channels::Table, Channels::Snowflake)
                            .from(Messages::Table, Messages::Channel)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .modify_column(ColumnDef::new(Messages::User).big_unsigned().not_null())
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("FK_messages_users")
                            .to(Users::Table, Users::Snowflake)
                            .from(Messages::Table, Messages::User)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Guilds::Table)
                    .drop_column(Guilds::Id)
                    .modify_column(
                        ColumnDef::new(Guilds::Snowflake)
                            .primary_key()
                            .big_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .modify_column(ColumnDef::new(Users::Guild).big_unsigned().not_null())
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("FK_users_guild")
                            .to(Guilds::Table, Guilds::Snowflake)
                            .from(Users::Table, Users::Guild)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .modify_column(ColumnDef::new(Channels::Guild).big_unsigned().not_null())
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("FK_channels_guild")
                            .to(Guilds::Table, Guilds::Snowflake)
                            .from(Channels::Table, Channels::Guild)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        // "ALTER TABLE <table_name> DROP CONSTRAINT <table_name>_pkey;"
        struct UnsetPrimaryKey<'a> {
            table_name: &'a str,
        }

        impl StatementBuilder for UnsetPrimaryKey<'_> {
            fn build(&self, db_backend: &DbBackend) -> Statement {
                let sql = match db_backend {
                    DbBackend::Postgres => format!("ALTER TABLE {} DROP CONSTRAINT {}_pkey;", self.table_name, self.table_name),
                    _ => panic!("Unsupported database backend"),
                };
                Statement::from_string(*db_backend, sql)
            }
        }
        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .drop_foreign_key(Alias::new("FK_channels_guild"))
                    .to_owned()
            ).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_foreign_key(Alias::new("FK_users_guild"))
                    .to_owned()
            ).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .drop_foreign_key(Alias::new("FK_messages_users"))
                    .drop_foreign_key(Alias::new("FK_messages_channels"))
                    .drop_foreign_key(Alias::new("FK_messages_replys_to"))
                    .to_owned()
            ).await?;

        manager.exec_stmt(UnsetPrimaryKey { table_name: "guilds" }).await?;
        manager.exec_stmt(UnsetPrimaryKey { table_name: "channels" }).await?;
        manager.exec_stmt(UnsetPrimaryKey { table_name: "users" }).await?;
        manager.exec_stmt(UnsetPrimaryKey { table_name: "messages" }).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Guilds::Table)
                    .add_column(
                        ColumnDef::new(Guilds::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .to_owned(),
            )
            .await?;


        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .add_column(
                        ColumnDef::new(Channels::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("channels_guilds")
                            .to(Guilds::Table, Guilds::Id)
                            .from(Channels::Table, Channels::Guild)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("users_guilds")
                            .to(Guilds::Table, Guilds::Id)
                            .from(Users::Table, Users::Guild)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .add_column(
                        ColumnDef::new(Messages::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("messages_messages")
                            .to(Messages::Table, Messages::Id)
                            .from(Messages::Table, Messages::ReplysTo)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("messages_channels")
                            .to(Channels::Table, Channels::Id)
                            .from(Messages::Table, Messages::Channel)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("messages_users")
                            .to(Users::Table, Users::Id)
                            .from(Messages::Table, Messages::User)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .get_foreign_key(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Guilds {
    Table,
    Id,
    Snowflake,
    Name,
    Score,
    MessageCount,
    UserCount,
}

#[derive(Iden)]
enum Channels {
    Table,
    Id,
    Snowflake,
    Name,
    Score,
    MessageCount,
    Guild,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Name,
    Snowflake,
    MessageCount,
    Score,
    Guild,
}

#[derive(Iden)]
enum Messages {
    Table,
    Id,
    Snowflake,
    Content,
    Score,
    ReplysTo,
    Channel,
    User,
}
