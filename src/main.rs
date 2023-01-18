// use crate::db::{
//     add_user, get_connection, get_user_from_id, store_message, RankChannel, RankGuild, RankMessage,
//     RankUser, DB,
// };
use crate::message_analyzer::score_message;
use crate::serenity::model::prelude::Message;
use async_recursion::async_recursion;
use entity::channels::ActiveModel as ChannelActiveModel;
use entity::guilds;
use entity::guilds::ActiveModel as GuildActiveModel;
use entity::messages::ActiveModel as MessageActiveModel;
use entity::prelude::Channels as ChannelsEntity;
use entity::prelude::Guilds as GuildsEntity;
use entity::prelude::Messages as MessagesEntity;
use entity::prelude::{Guilds, Messages, Users as UsersEntity};
use entity::users;
use entity::users::ActiveModel as UserActiveModel;
use entity::{channels, messages};
use env_file_reader::read_file;
use log::{debug, error, LevelFilter};
use migration::{Migrator, MigratorTrait};
use poise::serenity_prelude as serenity;
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::ColumnTrait;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, IntoActiveModel,
    NotSet, QueryFilter, TryIntoModel,
};
use std::borrow::BorrowMut;
use std::fs;
use std::fs::File;
use std::future::Future;
use std::ops::Deref;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;
// use tokio_rusqlite::Connection;

mod db;
mod handlers;
mod message_analyzer;

struct Handler;

struct ConnectionContainer;

// impl serenity::TypeMapKey for ConnectionContainer {
//     type Value = Connection;
// }

pub struct Data {
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
            handlers::message::handle_message(_ctx, &data, &msg)
                .await
                .expect("Failed to handle message");
        }
        _ => {}
    }
    log::debug!("event handler took {:?}", timer.elapsed());

    Ok(())
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
    let mut info_logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(fern::log_file(format!("log/info_{}.log", formatted_time))?);
    let mut debug_logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Debug)
        .chain(fern::log_file(format!("log/debug_{}.log", formatted_time))?)
        .chain(std::io::stdout());
    fern::Dispatch::new()
        // per-module overrides
        .level_for("serenity", LevelFilter::Off)
        .level_for("hyper", LevelFilter::Off)
        .level_for("poise", LevelFilter::Off)
        .level_for("tracing", LevelFilter::Off)
        .level_for("hs", LevelFilter::Off)
        .level_for("reqwest", LevelFilter::Off)
        .level_for("rustls", LevelFilter::Off)
        // Output to stdout, files, and other Dispatch configurations
        .chain(info_logger)
        .chain(debug_logger)
        .apply()
        .expect("Failed to initialize logger");
    // info level separate file

    // let conn = get_connection().unwrap();
    // let db = DB::new();
    // db::init_db(&conn)?;

    debug!("Starting up");

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

    let db = Database::connect(opt)
        .await
        .expect("Failed to connect to database");

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
