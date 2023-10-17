#![feature(async_fn_in_trait, async_closure)]

extern crate core;


use env_file_reader::read_file;
use log::{debug, LevelFilter};
use migration::{Migrator, MigratorTrait};
use poise::serenity_prelude as serenity;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use entity::guilds::Column::Snowflake as GuildSnowflake;
use entity::channels::Column::Snowflake as ChannelSnowflake;
use entity::users::Column::Snowflake as UserSnowflake;


use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, IntoActiveModel, QuerySelect, SelectColumns, Set};

use crate::handlers::message::handle_message;
use commands::messages;

use crate::commands::{leaderboard, stats, tests};
use std::time::Duration;
use serenity::model::prelude::{ChannelId, GuildId, UserId};
use tokio::sync::RwLock;
use tokio::time::Instant;
use entity::prelude::Guilds;
use crate::commands::leaderboard::leaderboard;
use crate::commands::stats::stats;
use crate::message_analyzer::score_message;
// use tokio_rusqlite::Connection;

mod commands;
mod db;
mod handlers;
mod logging;
mod message_analyzer;
mod scores;
mod common_words;

#[derive(Clone)]
pub struct Data {
    db: DatabaseConnection,
    guild_in_db: Arc<RwLock<HashSet<GuildId>>>,
    channel_in_db: Arc<RwLock<HashSet<ChannelId>>>,
    user_in_db: Arc<RwLock<HashSet<UserId>>>,
    common_words: Arc<HashSet<String>>,
}

unsafe impl Send for Data {}
unsafe impl Sync for Data {}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
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
            if !msg.author.bot {
                let score = score_message(msg, &data.db).await;

                handle_message(
                    score,
                    &_ctx.http.clone(),
                    data,
                    msg,
                    None,
                    &_ctx.cache.clone(),
                    false,
                    &data.guild_in_db,
                    &data.channel_in_db,
                    &data.user_in_db,
                )
                    .await
                    .expect("Failed to handle message");

                let mut a_guild = Guilds::find_by_id(msg.guild_id.unwrap().0 as i64)
                    .one(&data.db)
                    .await?
                    .unwrap()
                    .into_active_model();
                a_guild.score = Set(score + a_guild.score.unwrap());
                a_guild.message_count = Set(a_guild.message_count.unwrap() + 1);

                a_guild.update(&data.db).await?;

                let mut a_user = entity::users::Entity::find_by_id(msg.author.id.0 as i64)
                    .one(&data.db)
                    .await?
                    .unwrap()
                    .into_active_model();

                a_user.score = Set(score + a_user.score.unwrap());
                a_user.message_count = Set(a_user.message_count.unwrap() + 1);

                a_user.update(&data.db).await?;

                let mut a_channel = entity::channels::Entity::find_by_id(msg.channel_id.0 as i64)
                    .one(&data.db)
                    .await?
                    .unwrap()
                    .into_active_model();

                a_channel.score = Set(score + a_channel.score.unwrap());
                a_channel.message_count = Set(a_channel.message_count.unwrap() + 1);

                a_channel.update(&data.db).await?;
            }

        }
        _ => {}
    }
    debug!("event handler took {:?}", timer.elapsed());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    logging::setup_logging()?;

    debug!("Starting up");

    print!("\n db set up.. ");

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
        .connect_timeout(Duration::from_secs(20))
        .acquire_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(20))
        .max_lifetime(Duration::from_secs(20))
        .sqlx_logging(true)
        .sqlx_logging_level(LevelFilter::Info);

    let db = Database::connect(opt)
        .await
        .expect("Failed to connect to database");

    // Migrator::fresh(&db).await?; // (this is for when you want to reset the database)

    let guild_in_db = entity::guilds::Entity::find()
        .select_column(GuildSnowflake)
        .all(&db)
        .await?
        .into_iter()
        .map(|g| GuildId(g.snowflake as u64))
        .collect::<HashSet<_>>();

    let channel_in_db = entity::channels::Entity::find()
        .select_column(ChannelSnowflake)
        .all(&db)
        .await?
        .into_iter()
        .map(|c| ChannelId(c.snowflake as u64))
        .collect::<HashSet<_>>();

    let user_in_db = entity::users::Entity::find()
        .select_column(UserSnowflake)
        .all(&db)
        .await?
        .into_iter()
        .map(|u| UserId(u.snowflake as u64))
        .collect::<HashSet<_>>();

    println!("done!");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![messages::load_messages(), tests::test_progress_bar(), leaderboard::leaderboard(), stats::stats()],
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
                // poise::builtins::register_in_guild(
                //     ctx,
                //     &framework.options().commands,
                //     GuildId(729277347399991336),
                // )
                //     .await?;
                Ok(Data {
                    db,
                    guild_in_db: Arc::new(RwLock::new(guild_in_db)),
                    channel_in_db: Arc::new(RwLock::new(channel_in_db)),
                    user_in_db: Arc::new(RwLock::new(user_in_db)),
                    common_words: Arc::new(common_words::get_common_words()),
                })
            })
        });

    framework.run().await.unwrap();

    Ok(())
}
