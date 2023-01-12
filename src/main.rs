use std::borrow::BorrowMut;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use env_file_reader::read_file;
use log::LevelFilter;
use poise::serenity_prelude as serenity;
use tokio_rusqlite::Connection;
use crate::db::{add_user, create_db, get_connection, get_user_from_id, store_message};
use tokio::time::Instant;
use crate::message_analyzer::{score_message};


mod db;
mod message_analyzer;

struct Handler;

struct ConnectionContainer;

// impl serenity::TypeMapKey for ConnectionContainer {
//     type Value = Connection;
// }

struct Data {
    conn: Arc<Connection>
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
            println!("in servers: {:?}", _ctx.http.get_guilds(None, None).await?.iter().map(|g| g.name.as_str()).collect::<Vec<&str>>());
        }
        poise::Event::Message{new_message:msg} => {
            if msg.guild_id.is_some() {
                let guild_name = match msg.guild(&_ctx.cache) {
                    Some(g) => g.name,
                    None => _ctx.http.get_guild(msg.guild_id.unwrap().0).await?.name
                };
                let channel_name = match msg.channel((&_ctx.cache, _ctx.http.deref())).await?{
                    serenity::Channel::Guild(c) => c.name,
                    _ => "DM".to_string()
                };

                log::info!("[message] [{}:{}] [{}:{}] {}: {}", msg.guild_id.unwrap().0, guild_name, msg.channel_id, channel_name, msg.author.name, msg.content);
            }
            // let mut data = _framework.user_data().await.borrow_mut();

            // I have to clone all the values that I want to use in the sql statements because of the way serenity works. this doesn't really feel right to me.
            let author_id = msg.author.id.0;
            let author_tag = msg.author.tag().clone();

            let content = msg.content.clone();
            let timestamp = msg.timestamp.unix_timestamp();

            data.conn.call(move |conn| {
                if get_user_from_id(&conn, author_id).is_err() {
                    log::warn!("user not found in db");
                    add_user(&conn, author_id, &author_tag)?;
                }

                // let _ = _user_data.conn.call( |conn| store_message(&conn, msg.id.0, msg.content.clone(), msg.timestamp.unix_timestamp())).await?;
                println!("{:?}", store_message(&conn, author_id, content, timestamp)?);
                Ok::<(), Error>(())
            }).await?;
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
        .level(log::LevelFilter::Debug)
        // - and per-module overrides
        .level_for("serenity", log::LevelFilter::Off)
        .level_for("hyper", log::LevelFilter::Off)
        .level_for("poise", log::LevelFilter::Off)
        .level_for("tracing", log::LevelFilter::Off)
        // Output to stdout, files, and other Dispatch configurations
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        // Apply globally
        .apply()?;

    let conn = get_connection().unwrap();
    create_db(conn).unwrap();

    let env_variables = read_file("./TOKEN.env").expect("Failed to read .env file, does it exist?");

    let token = env_variables.get("TOKEN").expect("Failed to get TOKEN from .env file, did you set it?");

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
                Ok(Data {
                    conn: Arc::new(Connection::open("db.sqlite").await.unwrap()),
                })
            })
        });

    framework.run().await.unwrap();

    Ok(())
}
