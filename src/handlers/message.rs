use crate::message_analyzer::score_message;
use crate::serenity::model::prelude::Message;
use crate::{Data, Error};
use async_recursion::async_recursion;
use entity::guilds;

use std::collections::HashSet;

use entity::channels::{ActiveModel as ChannelActiveModel, Entity as ChannelEntity};
use entity::messages;
use entity::messages::{ActiveModel as MessageActiveModel, Entity as MessageEntity};
use entity::users::{ActiveModel as UserActiveModel, Entity as UserEntity};

use log::{error, trace, warn};

use crate::serenity::cache::Cache;
use crate::serenity::model::id::GuildId;
use poise::serenity_prelude as serenity;
use sea_orm::{DatabaseConnection, Set, TryIntoModel};

use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;

use entity::guilds::Model;
use entity::prelude::Guilds;
use migration::sea_orm::{ActiveModelTrait, EntityTrait};
use tokio::sync::RwLock;

#[allow(clippy::too_many_arguments)]
pub async fn handle_message(
    score: f32,
    http: &Arc<serenity::Http>,
    data: &Data,
    msg: &Message,
    guild_id: Option<GuildId>,
    cache: &Arc<Cache>,
    log: bool,
    guild_in_db: &RwLock<HashSet<u64>>,
    channel_in_db: &RwLock<HashSet<u64>>,
    user_in_db: &RwLock<HashSet<u64>>,
) -> Result<(), Error> {
    let _timer = Instant::now();

    if MessageEntity::find_by_id(msg.id.get() as i64)
        .one(&data.db)
        .await?
        .is_some()
    {
        return Ok(());
    }

    let guild_id = match guild_id {
        Some(guild_id) => guild_id,
        None => match msg.guild_id {
            Some(guild_id) => guild_id,
            None => {
                warn!("Message is not in a guild, ignoring");
                return Ok(());
            }
        },
    }
    .get();

    // dumb solution to https://github.com/rust-lang/rust/issues/87309
    let mut name = None;
    let guild = if !guild_in_db.read().await.contains(&guild_id) {
        if msg.guild(cache).is_some() {
            name = msg.guild(cache).map(|x| x.name.clone());
            None
        } else {
            match Guilds::find_by_id(guild_id as i64).one(&data.db).await? {
                Some(g) => {
                    guild_in_db.write().await.insert(guild_id);
                    Some(g)
                }
                None => {
                    let d_guild = http.get_guild(GuildId::new(guild_id)).await?;
                    let guild = guilds::ActiveModel {
                        snowflake: Set(guild_id as i64),
                        name: Set(d_guild.name),
                        score: Set(0.),
                        message_count: Set(0),
                        user_count: Set(0),
                    };
                    guild_in_db.write().await.insert(guild_id);
                    Some(guild.clone().insert(&data.db).await?)
                }
            }
        }
    } else if log {
        Some(
            match Guilds::find_by_id(guild_id as i64).one(&data.db).await? {
                Some(g) => g,
                None => {
                    error!("Guild not found in db");
                    return Err(Error::from("Guild not found in db"));
                }
            },
        )
    } else {
        None
    };

    // see https://github.com/rust-lang/rust/issues/87309
    if let Some(name) = name {
        guild_by_id(&data, guild_in_db, guild_id, name).await?;
    }

    let channel_name = if !channel_in_db.read().await.contains(&msg.channel_id.get()) {
        match ChannelEntity::find_by_id(msg.channel_id.get() as i64)
            .one(&data.db)
            .await?
        {
            Some(c) => {
                channel_in_db.write().await.insert(msg.channel_id.get());
                c.name
            }
            None => {
                trace!("Channel not found, creating");
                let channel_name = match msg.channel((cache, http.deref())).await? {
                    serenity::Channel::Guild(c) => c.name,
                    _ => "DM".to_string(),
                };
                let channel = ChannelActiveModel {
                    snowflake: Set(msg.channel_id.get() as i64),
                    name: Set(channel_name.clone()),
                    score: Set(0.),
                    message_count: Set(0),
                    guild: Set(guild_id as i64),
                };
                channel.insert(&data.db).await?.try_into_model()?;
                channel_in_db.write().await.insert(msg.channel_id.get());
                channel_name
            }
        }
    } else {
        match msg.channel((cache, http.deref())).await? {
            serenity::Channel::Guild(c) => c.name,
            _ => "DM".to_string(),
        }
    };

    if log && guild.is_some() {
        let guild_name = guild.unwrap().name;
        log::info!(
            "[message] [{}:{}] [{}:{}] {}: {}",
            guild_id,
            guild_name,
            msg.channel_id,
            channel_name,
            msg.author.name,
            msg.content
        );
    }

    if !user_in_db.read().await.contains(&msg.author.id.get()) {
        match UserEntity::find_by_id(msg.author.id.get() as i64)
            .one(&data.db)
            .await?
        {
            Some(_) => {
                user_in_db.write().await.insert(msg.author.id.get());
            }
            None => {
                let user = UserActiveModel {
                    snowflake: Set(msg.author.id.get() as i64),
                    name: Set(msg.author.name.clone()),
                    score: Set(0.),
                    message_count: Set(0),
                    guild: Set(guild_id as i64),
                };
                user_in_db.write().await.insert(msg.author.id.get());
                user.clone().insert(&data.db).await?;
            }
        };
    }

    let message = MessageActiveModel {
        snowflake: Set(msg.id.get() as i64),
        content: Set(msg.content.clone()),
        score: Set(score),
        user: Set(msg.author.id.get() as i64),
        channel: Set(msg.channel_id.get() as i64),
        replys_to: { Set(find_reply_to(&data.db, msg).await?) },
        timestamp: Set(msg.timestamp.naive_utc()),
    };

    message.insert(&data.db).await?;

    Ok(())
}

// this exists becuase of https://github.com/rust-lang/rust/issues/87309
async fn guild_by_id(
    data: &&Data,
    guild_in_db: &RwLock<HashSet<u64>>,
    guild_id: u64,
    guild_name: String,
) -> Result<Model, Error> {
    Ok(
        match Guilds::find_by_id(guild_id as i64).one(&data.db).await? {
            Some(guild) => {
                guild_in_db.write().await.insert(guild_id);
                guild
            }
            None => {
                let guild = guilds::ActiveModel {
                    snowflake: Set(guild_id as i64),
                    name: Set(guild_name),
                    score: Set(0.),
                    message_count: Set(0),
                    user_count: Set(0),
                };
                guild_in_db.write().await.insert(guild_id);
                guild.clone().insert(&data.db).await?
            }
        },
    )
}

#[async_recursion]
async fn find_reply_to(db: &DatabaseConnection, msg: &Message) -> Result<Option<i64>, Error> {
    let _timer = Instant::now();
    let reply_to = match &msg.referenced_message {
        Some(ref_msg) => {
            let reply_to = messages::Entity::find_by_id(ref_msg.id.get() as i64)
                .one(db)
                .await?;
            match reply_to {
                Some(reply_to) => Some(reply_to.snowflake),
                None => {
                    let reply_to = MessageActiveModel {
                        snowflake: Set(ref_msg.id.get() as i64),
                        content: Set(ref_msg.content.clone()),
                        score: Set(score_message(ref_msg, db).await),
                        user: Set(ref_msg.author.id.get() as i64),
                        channel: Set(ref_msg.channel_id.get() as i64),
                        replys_to: Set(find_reply_to(db, ref_msg).await?),
                        timestamp: Set(ref_msg.timestamp.naive_utc()),
                    };
                    reply_to.insert(db).await?;
                    Some(ref_msg.id.get() as i64)
                }
            }
        }
        None => None,
    };
    Ok(reply_to)
}
