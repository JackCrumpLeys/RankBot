// use crate::db::{
//     add_user, get_connection, get_user_from_id, store_message, RankChannel, RankGuild, RankMessage,
//     RankUser, DB,
// };
use crate::message_analyzer::score_message;
use crate::serenity::model::prelude::Message;
use env_file_reader::read_file;
use log::{error, LevelFilter};
use poise::serenity_prelude as serenity;
use std::borrow::BorrowMut;
use std::fs;
use std::fs::File;
use std::future::Future;
use std::ops::Deref;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, IntoActiveModel, NotSet, QueryFilter, TryIntoModel};
use sea_orm::ActiveValue::{Set, Unchanged};
use tokio::time::Instant;
use entity::prelude::{Guilds, Messages, Users as UsersEntity};
use entity::users::ActiveModel as UserActiveModel;
use entity::guilds::ActiveModel as GuildActiveModel;
use entity::messages::ActiveModel as MessageActiveModel;
use entity::channels::ActiveModel as ChannelActiveModel;
use entity::prelude::Messages as MessagesEntity;
use entity::prelude::Guilds as GuildsEntity;
use entity::prelude::Channels as ChannelsEntity;
use entity::{channels, messages};
use entity::users;
use entity::guilds;
use migration::{Migrator, MigratorTrait};
use sea_orm::ColumnTrait;
use async_recursion::async_recursion;
// use tokio_rusqlite::Connection;

mod db;
mod message_analyzer;

struct Handler;

struct ConnectionContainer;

// impl serenity::TypeMapKey for ConnectionContainer {
//     type Value = Connection;
// }

struct Data {
    db: DatabaseConnection,
}

unsafe impl Send for Data {}
unsafe impl Sync for Data {}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// lazy_static! {
//
// }

async fn event_event_handler(
    _ctx: &serenity::Context,
    event: &poise::Event<'_>,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    let timer = Instant::now();
    match event {
        poise::Event::Ready { data_about_bot } => {
            println!("{} is connected!", data_about_bot.user.name);
            println!(
                "in servers: {:?}",
                _ctx.http
                    .get_guilds(None, None)
                    .await?
                    .iter()
                    .map(|g| g.name.as_str())
                    .collect::<Vec<&str>>()
            );
        }
        poise::Event::Message { new_message: msg } => {
            let guild_name = match msg.guild(&_ctx.cache) {
                Some(g) => g.name,
                None => _ctx.http.get_guild(msg.guild_id.unwrap().0).await?.name,
            };
            let channel_name = match msg.channel((&_ctx.cache, _ctx.http.deref())).await? {
                serenity::Channel::Guild(c) => c.name,
                _ => "DM".to_string(),
            };

            if msg.guild_id.is_some() {
                log::info!(
                    "[message] [{}:{}] [{}:{}] {}: {}",
                    msg.guild_id.unwrap().0,
                    guild_name,
                    msg.channel_id,
                    channel_name,
                    msg.author.name,
                    msg.content
                );
            }
            let guild = match Guilds::find().filter(guilds::Column::Snowflake.eq(msg.guild_id.unwrap().0)).one(&data.db).await? {
                Some(mut g) => {
                    let mut g = g.into_active_model();
                    g.score = Set(score_message(&msg.content) + g.score.unwrap());
                    g.message_count = Set(g.message_count.unwrap() + 1);
                    g.clone().update(&data.db).await?
                },
                None => {
                    let g = guilds::ActiveModel {
                        id: NotSet,
                        snowflake: Set(msg.guild_id.unwrap().0 as i64),
                        name: Set(guild_name),
                        score: Set(score_message(&msg.content)),
                        message_count: Set(1),
                        user_count: Set(1),
                    };
                    dbg!(g.clone().insert(&data.db).await?)
                }
            };
            let user = match UsersEntity::find().filter(users::Column::Snowflake.eq(msg.author.id.0)).one(&data.db).await? {
                    Some(u) => {
                        let mut u = u.into_active_model();
                        u.score = Set(score_message(&msg.content) + u.score.unwrap());
                        u.message_count = Set(u.message_count.unwrap() + 1);
                        u.clone().update(&data.db).await?
                    },
                    None => {
                        let user = UserActiveModel {
                            id: NotSet,
                            snowflake: Set(msg.author.id.0 as i64),
                            name: Set(msg.author.tag()),
                            score: Set(score_message(&msg.content)),
                            message_count: Set(1),
                            guild: Set(guild.id),
                        };
                        user.insert(&data.db).await?.try_into_model()?
                    }
            };
            let channel = match ChannelsEntity::find().filter(entity::channels::Column::Snowflake.eq(msg.channel_id.0)).one(&data.db).await? {
                Some(c) => {
                    let mut c = c.into_active_model();
                    c.score = Set(score_message(&msg.content) + c.score.unwrap());
                    c.message_count = Set(c.message_count.unwrap() + 1);
                    c.clone().update(&data.db).await?
                },
                None => {
                    let channel = ChannelActiveModel {
                        id: NotSet,
                        snowflake: Set(msg.channel_id.0 as i64),
                        name: Set(channel_name),
                        score: Set(score_message(&msg.content)),
                        message_count: Set(1),
                        guild: Set(guild.id),
                    };
                    channel.insert(&data.db).await?.try_into_model()?
                }
            };
            let message = MessageActiveModel {
                id: NotSet,
                snowflake: Set(msg.id.0 as i64),
                content: Set(msg.content.clone()),
                score: Set(score_message(&msg.content)),
                user: Set(user.id),
                channel: Set(channel.id),
                replys_to: {
                    Set(find_reply_to(&data.db, &_ctx, &msg, channel, user).await?)
                },
            };

            message.save(&data.db).await?;
        }
        _ => {}
    }
    log::debug!("event handler took {:?}", timer.elapsed());

    Ok(())
}

#[async_recursion]
async fn find_reply_to(db: &DatabaseConnection, ctx: &serenity::Context, msg: &Message, channel: channels::Model, user: users::Model) -> Result<Option<i32>, Error>{
        match Messages::find().filter(messages::Column::Snowflake.eq(msg.id.0)).one(db).await? {
            Some(msg) => Ok(Some(msg.id)),
            None => {
                match msg.message_reference {
                    Some(ref r) => {
                        match r.message_id {
                            Some(msg_id) => {
                                match ctx.http.get_message(r.channel_id.0, r.message_id.unwrap().0).await {
                                    Ok(m) => {
                                        let message = MessageActiveModel {
                                            id: NotSet,
                                            snowflake: Set(m.id.0 as i64),
                                            content: Set(m.content.clone()),
                                            score: Set(score_message(&m.content)),
                                            user: Set(user.id),
                                            channel: Set(channel.id),
                                            replys_to: Set(find_reply_to(db, ctx, &m, channel, user).await?),
                                        };
                                        let db_msg = dbg!(message.insert(db).await?);
                                        Ok(Some(db_msg.id))
                                    },
                                    _ => {
                                        error!("Could not find message that was replied to from message {}", msg.id.0);
                                        Ok(None)
                                    },
                                }
                            },
                            _ => {
                                error!("Could not find message id of message that was replied to from message {}", msg.id.0);
                                Ok(None)
                            },
                        }
                    }
                    _ => Ok(None),
                }
            }
        }
}

/// Displays your or another user's account creation date
#[poise::command(slash_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let formatted_time = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        // Add blanket level filter -
        .level(LevelFilter::Debug)
        // - and per-module overrides
        .level_for("serenity", log::LevelFilter::Off)
        .level_for("hyper", log::LevelFilter::Off)
        .level_for("poise", log::LevelFilter::Off)
        .level_for("tracing", log::LevelFilter::Off)
        // Output to stdout, files, and other Dispatch configurations
        .chain(std::io::stdout())
        .chain(fern::log_file("log/debug.log")?)
        .chain(fern::log_file(format!("log/debug-{}.log", formatted_time))?)
        // info level separate file
        .level(LevelFilter::Info)
        .chain(fern::log_file("log/info.log")?)
        .chain(fern::log_file(format!("log/info-{}.log", formatted_time))?)
        // Apply globally
        .apply().expect("Failed to initialize logging");

    // let conn = get_connection().unwrap();
    // let db = DB::new();
    // db::init_db(&conn)?;


    let env_variables = read_file("./auth.env").expect("Failed to read .env file, does it exist?");

    let token = env_variables
        .get("TOKEN")
        .expect("Failed to get TOKEN from .env file, did you set it?");

    let db_url = env_variables
        .get("DATABASE_URL")
        .expect("Failed to get DATABASE_URL from .env file, did you set it?");


    // let db_url = "sqlite://./db.db"; // you have to provide a database BEFORE running the bot

    let mut opt = ConnectOptions::new(db_url.to_owned());
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(LevelFilter::Info);


    let db = Database::connect(opt).await.expect("Failed to connect to database");

    Migrator::fresh(&db).await?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![age()],
            event_handler: |ctx, event, framework, user_data| {
                Box::pin(event_event_handler(ctx, event, framework, user_data))
            },
            ..Default::default()
        })
        .token(token)
        .intents(serenity::GatewayIntents::all())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { db })
            })
        });

    framework.run().await.unwrap();

    Ok(())
}
