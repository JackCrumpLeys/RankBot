use std::collections::HashMap;
use crate::message_analyzer::score_message;
use crate::serenity::model::prelude::Message;
use crate::{Data, Error};
use async_recursion::async_recursion;
use entity::channels::ActiveModel as ChannelActiveModel;
use entity::{channels, guilds, users};

use entity::messages::ActiveModel as MessageActiveModel;
use entity::prelude::Channels as ChannelsEntity;

use entity::messages;
use entity::prelude::{Guilds, Messages, Users as UsersEntity};
use entity::users::ActiveModel as UserActiveModel;

use log::{debug, error, trace, warn};

use poise::{serenity_prelude as serenity, serenity_prelude};
use sea_orm::ActiveValue::Set;
use sea_orm::ColumnTrait;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, NotSet, QueryFilter,
    TryIntoModel,
};

use crate::serenity::cache::Cache;
use crate::serenity::model::id::GuildId;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;
use crate::serenity::model::channel::MessageReference;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};
use crate::serenity::model::prelude::MessageId;

pub async fn handle_message(
    http: &Arc<serenity::Http>,
    data: &Data,
    msg: &Message,
    guild_id: Option<GuildId>,
    cache: &Arc<Cache>,
    log: bool,
    message_cache: Arc<RwLock<HashMap<MessageId, Message>>>,
) -> Result<(), Error> {
    let timer = Instant::now();
    trace!("Handling message: {:?}", msg);
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

    let guild = match msg.guild(&cache) { // this is very messy and could be better :/
        Some(d_guild) => {
            match Guilds::find().filter(guilds::Column::Snowflake.eq(guild_id.0)).one(&data.db).await? {
                Some(guild) => {
                    let mut guild = guild.into_active_model();
                    guild.score = Set(score_message(&msg.content) + guild.score.unwrap());
                    guild.message_count = Set(guild.message_count.unwrap() + 1);
                    guild.clone().update(&data.db).await?
                },
                None => {
                    let guild = guilds::ActiveModel {
                        id: NotSet,
                        snowflake: Set(guild_id.0 as i64),
                        name: Set(d_guild.name),
                        score: Set(score_message(&msg.content)),
                        message_count: Set(1),
                        user_count: Set(1),
                    };
                    guild.clone().insert(&data.db).await?
                }
            }
        },
        None => {
            match Guilds::find().filter(guilds::Column::Snowflake.eq(guild_id.0)).one(&data.db).await? {
                Some(g) => {
                    let mut guild = g.into_active_model();
                    guild.score = Set(score_message(&msg.content) + guild.score.unwrap());
                    guild.message_count = Set(guild.message_count.unwrap() + 1);
                    guild.clone().update(&data.db).await?
                },
                None => {
                    let d_guild = http.get_guild(guild_id.0).await?;
                    let guild = guilds::ActiveModel {
                        id: NotSet,
                        snowflake: Set(guild_id.0 as i64),
                        name: Set(d_guild.name),
                        score: Set(score_message(&msg.content)),
                        message_count: Set(1),
                        user_count: Set(1),
                    };
                    guild.clone().insert(&data.db).await?
                }
            }
        },
    };
    let guild_name = guild.name;

    let channel = match ChannelsEntity::find()
        .filter(channels::Column::Snowflake.eq(msg.channel_id.0 as i64))
        .one(&data.db)
        .await?
    {
        Some(c) => {
            let mut c = c.into_active_model();
            c.score = Set(score_message(&msg.content) + c.score.unwrap());
            c.message_count = Set(c.message_count.unwrap() + 1);
            c.clone().update(&data.db).await?
        }
        None => {
            trace!("Channel not found, creating");
            let channel_name = match msg.channel((cache, http.deref())).await? {
                serenity::Channel::Guild(c) => c.name,
                _ => "DM".to_string(),
            };
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
    let channel_name = channel.name.clone();

    if log {
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
    let user = match UsersEntity::find()
        .filter(users::Column::Snowflake.eq(msg.author.id.0))
        .one(&data.db)
        .await?
    {
        Some(u) => {
            let mut u = u.into_active_model();
            u.score = Set(score_message(&msg.content) + u.score.unwrap());
            u.message_count = Set(u.message_count.unwrap() + 1);
            u.clone().update(&data.db).await?
        }
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

    let message = MessageActiveModel {
        id: NotSet,
        snowflake: Set(msg.id.0 as i64),
        content: Set(msg.content.clone()),
        score: Set(score_message(&msg.content)),
        user: Set(user.id),
        channel: Set(channel.id),
        replys_to: { Set(find_reply_to(&data.db, msg, &channel, user, http, message_cache.read().await).await?) },
    };

    message.save(&data.db).await?;

    trace!("Message handled in {:?}", timer.elapsed());
    Ok(())
}

#[async_recursion]
async fn find_reply_to(
    db: &DatabaseConnection,
    msg: &Message,
    channel: &channels::Model,
    user: users::Model,
    http: &Arc<serenity::Http>,
    message_cache: RwLockReadGuard<'async_recursion, HashMap<MessageId, Message>>,
) -> Result<Option<i32>, Error> {
    match message_cache.get(&msg.id) {
        Some(m) => {
            match &m.message_reference {
                Some(msg_ref) => {
                    let reply_to = match msg_ref.message_id {
                        Some(msg_id) => {
                            match message_cache.get(&msg_id) {
                                Some(m) => {
                                    match UsersEntity::find()
                                        .filter(users::Column::Snowflake.eq(m.author.id.0))
                                        .one(db)
                                        .await?
                                    {
                                        Some(u) => Some(u.id),
                                        None => {
                                            let user = UserActiveModel {
                                                id: NotSet,
                                                snowflake: Set(m.author.id.0 as i64),
                                                name: Set(m.author.tag()),
                                                score: Set(0),
                                                message_count: Set(0),
                                                guild: Set(channel.guild),
                                            };
                                            Some(user.insert(db).await?.try_into_model()?.id)
                                        }
                                    }
                                }
                                None => {
                                    let m = http.get_message(msg_ref.channel_id.0, msg_id.0).await?;
                                    match UsersEntity::find()
                                        .filter(users::Column::Snowflake.eq(m.author.id.0))
                                        .one(db)
                                        .await?
                                    {
                                        Some(u) => Some(u.id),
                                        None => {
                                            let user = UserActiveModel {
                                                id: NotSet,
                                                snowflake: Set(m.author.id.0 as i64),
                                                name: Set(m.author.tag()),
                                                score: Set(0),
                                                message_count: Set(0),
                                                guild: Set(channel.guild),
                                            };
                                            Some(user.insert(db).await?.try_into_model()?.id)
                                        }
                                    }
                                }
                            }
                        }
                        None => None,
                    };
                    Ok(reply_to)
                }
                None => {
                    return Ok(None);
                }
            }
        }
        None => {
            match Messages::find()
                .filter(messages::Column::Snowflake.eq(msg.id.0))
                .one(db)
                .await?
            {
                Some(msg) => Ok(Some(msg.id)),
                None => match msg.message_reference {
                    Some(ref r) => match r.message_id {
                        Some(_msg_id) => {
                            match http
                                .get_message(r.channel_id.0, r.message_id.unwrap().0)
                                .await
                            {
                                Ok(m) => {
                                    let message = MessageActiveModel {
                                        id: NotSet,
                                        snowflake: Set(m.id.0 as i64),
                                        content: Set(m.content.clone()),
                                        score: Set(score_message(&m.content)),
                                        user: Set(user.id),
                                        channel: Set(channel.id),
                                        replys_to: Set(find_reply_to(db, &m, channel, user, http, message_cache).await?),
                                    };
                                    let db_msg = message.insert(db).await?;
                                    Ok(Some(db_msg.id))
                                }
                                _ => {
                                    error!(
                                "Could not find message that was replied to from message {}",
                                msg.id.0
                            );
                                    Ok(None)
                                }
                            }
                        }
                        _ => {
                            error!(
                        "Could not find message id of message that was replied to from message {}",
                        msg.id.0
                    );
                            Ok(None)
                        }
                    },
                    _ => Ok(None),
                },
            }
        }
    }
}
