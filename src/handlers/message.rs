use crate::message_analyzer::score_message;
use crate::serenity::model::prelude::Message;
use crate::{Data, Error};
use async_recursion::async_recursion;
use entity::{channels, guilds, users};
use std::collections::{HashMap, HashSet};

use entity::channels::{ActiveModel as ChannelActiveModel, Entity as ChannelEntity};
use entity::messages;
use entity::messages::{ActiveModel as MessageActiveModel, Entity as MessageEntity};
use entity::users::{ActiveModel as UserActiveModel, Entity as UserEntity};
use sea_orm::ColumnTrait;
use sea_orm::{IntoActiveModel, QueryFilter};

use log::{error, trace, warn};

use crate::serenity::cache::Cache;
use crate::serenity::model::id::GuildId;
use poise::{serenity_prelude as serenity, serenity_prelude};
use sea_orm::{DatabaseConnection, NotSet, Set, TryIntoModel};
use serenity::model::guild::Guild;
use serenity::model::prelude::{ChannelId, UserId};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;

use crate::serenity::model::prelude::MessageId;
use entity::prelude::Guilds;
use migration::sea_orm::{ActiveModelTrait, EntityTrait};
use tokio::sync::{RwLock, RwLockReadGuard};

pub async fn handle_message(
    score: f32,
    http: &Arc<serenity::Http>,
    data: &Data,
    msg: &Message,
    guild_id: Option<GuildId>,
    cache: &Arc<Cache>,
    log: bool,
    guild_in_db: &RwLock<HashSet<GuildId>>,
    channel_in_db: &RwLock<HashSet<ChannelId>>,
    user_in_db: &RwLock<HashSet<UserId>>,
) -> Result<(), Error>{
    let timer = Instant::now();

    if MessageEntity::find_by_id(msg.id.0 as i64)
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
    };

    let guild = if !guild_in_db.read().await.contains(&guild_id) {
        Some(match msg.guild(cache) {
            // this is very messy and could be better :/
            Some(d_guild) => {
                match Guilds::find_by_id(d_guild.id.0 as i64)
                    .one(&data.db)
                    .await?
                {
                    Some(guild) => {
                        guild_in_db.write().await.insert(guild_id);
                        guild
                    }
                    None => {
                        let guild = guilds::ActiveModel {
                            snowflake: Set(guild_id.0 as i64),
                            name: Set(d_guild.name),
                            score: Set(0.),
                            message_count: Set(0),
                            user_count: Set(0),
                        };
                        guild_in_db.write().await.insert(guild_id);
                        guild.clone().insert(&data.db).await?
                    }
                }
            }
            None => match Guilds::find_by_id(guild_id.0 as i64).one(&data.db).await? {
                Some(g) => {
                    guild_in_db.write().await.insert(guild_id);
                    g
                }
                None => {
                    let d_guild = http.get_guild(guild_id.0).await?;
                    let guild = guilds::ActiveModel {
                        snowflake: Set(guild_id.0 as i64),
                        name: Set(d_guild.name),
                        score: Set(0.),
                        message_count: Set(0),
                        user_count: Set(0),
                    };
                    guild_in_db.write().await.insert(guild_id);
                    guild.clone().insert(&data.db).await?
                }
            },
        })
    } else {
        if log {
            Some(
                match Guilds::find_by_id(guild_id.0 as i64)
                    .one(&data.db)
                    .await?
                {
                    Some(g) => g,
                    None => {
                        error!("Guild not found in db");
                        return Err(Error::from("Guild not found in db"));
                    }
                },)
        } else {
            None
        }
    };

    let channel_name = if !channel_in_db.read().await.contains(&msg.channel_id) {
        match ChannelEntity::find_by_id(msg.channel_id.0 as i64)
            .one(&data.db)
            .await?
        {
            Some(c) => {
                channel_in_db.write().await.insert(msg.channel_id);
                c.name
            }
            None => {
                trace!("Channel not found, creating");
                let channel_name = match msg.channel((cache, http.deref())).await? {
                    serenity::Channel::Guild(c) => c.name,
                    _ => "DM".to_string(),
                };
                let channel = ChannelActiveModel {
                    snowflake: Set(msg.channel_id.0 as i64),
                    name: Set(channel_name.clone()),
                    score: Set(0.),
                    message_count: Set(0),
                    guild: Set(guild_id.0 as i64),
                };
                channel.insert(&data.db).await?.try_into_model()?;
                channel_in_db.write().await.insert(msg.channel_id);
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
            guild_id.0,
            guild_name,
            msg.channel_id,
            channel_name,
            msg.author.name,
            msg.content
        );
    }

    if !user_in_db.read().await.contains(&msg.author.id) {
        match UserEntity::find_by_id(msg.author.id.0 as i64).one(&data.db).await? {
            Some(_) => {
                user_in_db.write().await.insert(msg.author.id);
            },
            None => {
                let user = UserActiveModel {
                    snowflake: Set(msg.author.id.0 as i64),
                    name: Set(msg.author.name.clone()),
                    score: Set(0.),
                    message_count: Set(0),
                    guild: Set(guild_id.0 as i64),
                };
                user_in_db.write().await.insert(msg.author.id);
                user.clone().insert(&data.db).await?;
            }
        };
    }


    let message = MessageActiveModel {
        snowflake: Set(msg.id.0 as i64),
        content: Set(msg.content.clone()),
        score: Set(score),
        user: Set(msg.author.id.0 as i64),
        channel: Set(msg.channel_id.0 as i64),
        replys_to: {
            Set(find_reply_to(&data.db, msg, http).await?)
        },
        timestamp: Set(msg.timestamp.naive_utc()),
    };

    message.insert(&data.db).await?;

    Ok(())
}

#[async_recursion]
async fn find_reply_to(
    db: &DatabaseConnection,
    msg: &Message,
    http: &Arc<serenity::Http>,
) -> Result<Option<i64>, Error> {
    let timer = Instant::now();
    let reply_to = match &msg.referenced_message {
        Some(ref_msg) => {
            let reply_to = messages::Entity::find_by_id(ref_msg.id.0 as i64)
                .one(db)
                .await?;
            match reply_to {
                Some(reply_to) => Some(reply_to.snowflake),
                None => {
                    let reply_to = MessageActiveModel {
                        snowflake: Set(ref_msg.id.0 as i64),
                        content: Set(ref_msg.content.clone()),
                        score: Set(score_message(&ref_msg, db).await),
                        user: Set(ref_msg.author.id.0 as i64),
                        channel: Set(ref_msg.channel_id.0 as i64),
                        replys_to: Set(find_reply_to(db, &ref_msg, http).await?),
                        timestamp: Set(ref_msg.timestamp.naive_utc()),
                    };
                    reply_to.insert(db).await?;
                    Some(ref_msg.id.0 as i64)
                }
            }
        }
        None => None,
    };
    Ok(reply_to)
}
