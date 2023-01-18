use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::ColumnRef::Column;

#[derive(DeriveMigrationName)]
pub struct Migration;

// old code:
//         conn.execute(
//             "CREATE TABLE IF NOT EXISTS guilds (
//                 id INTEGER PRIMARY KEY,
//                 snowflake INTEGER NOT NULL,
//                 name TEXT NOT NULL,
//                 score INTEGER NOT NULL,
//                 message_count INTEGER NOT NULL,
//                 user_count INTEGER NOT NULL
//             )",
//             params![],
//         )?;
//         conn.execute(
//             "CREATE TABLE IF NOT EXISTS channels (
//                 id INTEGER PRIMARY KEY,
//                 snowflake INTEGER NOT NULL,
//                 name TEXT NOT NULL,
//                 score INTEGER NOT NULL,
//                 message_count INTEGER NOT NULL,
//                 guild INTEGER NOT NULL,
//                 FOREIGN KEY(guild) REFERENCES guilds(id)
//             )",
//             params![],
//         )?;
//         conn.execute(
//             "CREATE TABLE IF NOT EXISTS users (
//                 id INTEGER PRIMARY KEY,
//                 snowflake INTEGER NOT NULL,
//                 message_count INTEGER NOT NULL,
//                 score INTEGER NOT NULL,
//                 guild INTEGER NOT NULL,
//                 FOREIGN KEY(guild) REFERENCES guilds(id)
//             )",
//             params![],
//         )?;
//         conn.execute(
//             "CREATE TABLE IF NOT EXISTS messages (
//                 id INTEGER PRIMARY KEY,
//                 snowflake INTEGER NOT NULL,
//                 content TEXT NOT NULL,
//                 score INTEGER NOT NULL,
//                 replys_to INTEGER,
//                 channel INTEGER NOT NULL,
//                 user INTEGER NOT NULL,
//                 FOREIGN KEY(replys_to) REFERENCES messages(id),
//                 FOREIGN KEY(channel) REFERENCES channels(id),
//                 FOREIGN KEY(user) REFERENCES users(id)
//             )",
//             params![],
//         )?;

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

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // migrations

        manager
            .create_table( // create table
                Table::create()
                    .table(
                        Guilds::Table,
                    )
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Guilds::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(Guilds::Snowflake)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Guilds::Name)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Guilds::Score)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Guilds::MessageCount)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Guilds::UserCount)
                            .integer()
                            .not_null(),
                    )
                    .to_owned()
            )
            .await?;

        manager.create_table(
            Table::create()
                .table(
                    Channels::Table,
                )
                .if_not_exists()
                .col(
                    ColumnDef::new(Channels::Id)
                        .integer()
                        .not_null()
                        .primary_key()
                        .auto_increment(),
                )
                .col(
                    ColumnDef::new(Channels::Snowflake)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Channels::Name)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Channels::Score)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Channels::MessageCount)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Channels::Guild)
                        .integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("channels_guilds")
                        .to(Guilds::Table, Guilds::Id)
                        .from(Channels::Table, Channels::Guild)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .to_owned()
        )
        .await?;

        manager.create_table(
            Table::create()
                .table(
                    Users::Table,
                )
                .if_not_exists()
                .col(
                    ColumnDef::new(Users::Id)
                        .integer()
                        .not_null()
                        .primary_key()
                        .auto_increment(),
                )
                .col(
                    ColumnDef::new(Users::Snowflake)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Users::Name)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Users::MessageCount)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Users::Score)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Users::Guild)
                        .integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("users_guilds")
                        .to(Guilds::Table, Guilds::Id)
                        .from(Users::Table, Users::Guild)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(
                    Messages::Table,
                )
                .if_not_exists()
                .col(
                    ColumnDef::new(Messages::Id)
                        .integer()
                        .not_null()
                        .primary_key()
                        .auto_increment(),
                )
                .col(
                    ColumnDef::new(Messages::Snowflake)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Messages::Content)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Messages::Score)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Messages::ReplysTo)
                        .integer(),
                )
                .col(
                    ColumnDef::new(Messages::Channel)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(Messages::User)
                        .integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("messages_messages")
                        .to(Messages::Table, Messages::Id)
                        .from(Messages::Table, Messages::ReplysTo)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("messages_channels")
                        .to(Channels::Table, Channels::Id)
                        .from(Messages::Table, Messages::Channel)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("messages_users")
                        .to(Users::Table, Users::Id)
                        .from(Messages::Table, Messages::User)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {


        manager
            .drop_table(
                Table::drop()
                    .table(
                        Messages::Table,
                    )
                    .if_exists()
                    .to_owned(),
            )
        .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(
                        Users::Table,
                    )
                    .if_exists()
                    .to_owned(),
            )
        .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(
                        Channels::Table,
                    )
                    .if_exists()
                    .to_owned(),
            )
        .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(
                        Guilds::Table,
                    )
                    .if_exists()
                    .to_owned(),
            )
        .await?;

        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Post {
    Table,
    Id,
    Title,
    Text,
}
