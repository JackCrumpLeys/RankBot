// use crate::db::{
//     add_user, get_connection, get_user_from_id, store_message, RankChannel, RankGuild, RankMessage,
//     RankUser, DB,
// };
use crate::message_analyzer::score_message;
use env_file_reader::read_file;
use log::LevelFilter;
use poise::serenity_prelude as serenity;
use std::borrow::BorrowMut;
use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tokio::time::Instant;
use migration::{Migrator, MigratorTrait};
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
            if msg.guild_id.is_some() {
                let guild_name = match msg.guild(&_ctx.cache) {
                    Some(g) => g.name,
                    None => _ctx.http.get_guild(msg.guild_id.unwrap().0).await?.name,
                };
                let channel_name = match msg.channel((&_ctx.cache, _ctx.http.deref())).await? {
                    serenity::Channel::Guild(c) => c.name,
                    _ => "DM".to_string(),
                };

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
            // let mut data = _framework.user_data().await.borrow_mut();
            //
            // // I have to clone all the values that I want to use in the sql statements because of the way serenity works. this doesn't really feel right to me.
            // let author_id = msg.author.id;
            // let author_tag = msg.author.tag().clone();
            //
            // let content = msg.content.clone();
            //
            // let user_cache = data.db.users.write().await;
            // let message_cache = data.db.messages.write().await;
            //
            // let reply_to = msg.message_reference.clone();
            // let guild_id = msg.guild_id.clone();
            // let channel_id = msg.channel_id.clone();
            //
            // let mut user = RankUser::new(author_id);
            // let message_score = score_message(&content);
            //
            // if user.update(&data.db.conn, user_cache, true).await.is_err() {
            //     log::warn!("user not found in db, making new one");
            //
            //     user.message_count = Some(1);
            //     user.score = Some(message_score);
            //     user.guild = Some(RankGuild::new(guild_id.unwrap()));
            // }
            //
            // // let _ = _user_data.conn.call( |conn| store_message(&conn, msg.id.0, msg.content.clone(), msg.timestamp.unix_timestamp())).await?;
            // let mut message = RankMessage::new(msg.id);
            // message.content = Some(content);
            // message.score = Some(message_score);
            // message.replys_to = match reply_to {
            //     Some(r) => {
            //         match r.message_id {
            //             Some(msg_id) => Some(Box::new(RankMessage::new(r.message_id.unwrap()))),
            //             _ => None,
            //         }
            //     }
            //     _ => None,
            // };
            // message.channel = Some(RankChannel::new(channel_id));
            // message.author = Some(user.clone());
            // message.update(&data.db.conn, message_cache, true).await?;
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

    let db_url = "sqlite://./db.db"; // you have to provide a database BEFORE running the bot

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

    Migrator::up(&db, None).await?;

    let env_variables = read_file("./TOKEN.env").expect("Failed to read .env file, does it exist?");

    let token = env_variables
        .get("TOKEN")
        .expect("Failed to get TOKEN from .env file, did you set it?");

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
