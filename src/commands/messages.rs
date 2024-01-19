use crate::{Context, Error};
use async_iterator::Iterator;
use indicatif::ProgressIterator;
use log::warn;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, ModelTrait, SelectColumns};
use serenity::builder::GetMessages;
use serenity::model::channel::{GuildChannel, PrivateChannel};
use serenity::model::id::MessageId;
use serenity::model::prelude::{GuildId, Message};
use std::collections::HashMap;
use std::iter::{IntoIterator as StdIntoIterator, Iterator as stdIter};
use std::sync::Arc;

use crate::handlers::message::handle_message;
use crate::message_analyzer::score_message;
use entity::prelude::{Channels, Guilds};
use tokio::time::Instant;

#[allow(dead_code)]
pub(crate) enum HistoryChannel<'a> {
    Guild(&'a GuildChannel),
    Private(&'a PrivateChannel),
}

struct HistoryIterator<'a> {
    channel: &'a HistoryChannel<'a>,
    http: Arc<serenity::http::Http>,
    last_message_id: Option<MessageId>,
    limit: u8,
    current: u8,
    before: Option<u64>,
    after: Option<u64>,
    around: Option<u64>,
    messages: Vec<Message>,
}

async fn fill_messages(
    channel: &HistoryChannel<'_>,
    http: Arc<serenity::http::Http>,
    last_message_id: Option<MessageId>,
    limit: u8,
    before: Option<u64>,
    after: Option<u64>,
    around: Option<u64>,
) -> Result<(Vec<Message>, Option<MessageId>), Error> {
    let mut messages = Vec::new();
    let mut last_message_id = last_message_id;
    let messages_gotten = match channel {
        HistoryChannel::Private(channel) => {
            // channel
            //     .messages(http, |retriever| {
            //         if let Some(last_message_id) = last_message_id {
            //             retriever.before(last_message_id);
            //         } else if let Some(before) = before {
            //             retriever.before(before);
            //         }
            //
            //         if let Some(after) = after {
            //             retriever.after(after);
            //         }
            //
            //         if let Some(around) = around {
            //             retriever.around(around);
            //         }
            //
            //         retriever.limit(limit)
            //     })
            //     .await?
            let mut config = GetMessages::default();
            if let Some(last_message_id) = last_message_id {
                config = config.before(last_message_id);
            } else if let Some(before) = before {
                config = config.before(before);
            }

            if let Some(after) = after {
                config = config.after(after);
            }

            if let Some(around) = around {
                config = config.around(around);
            }

            config = config.limit(limit);

            channel.messages(&http, config).await?
        }
        HistoryChannel::Guild(channel) => {
            let mut config = GetMessages::default();
            if let Some(last_message_id) = last_message_id {
                config = config.before(last_message_id);
            } else if let Some(before) = before {
                config = config.before(before);
            }

            if let Some(after) = after {
                config = config.after(after);
            }

            if let Some(around) = around {
                config = config.around(around);
            }

            config = config.limit(limit);

            channel.messages(&http, config).await?
        }
    };

    // messages_gotten.reverse();

    for message in messages_gotten {
        last_message_id = Some(message.id);
        messages.push(message);
    }

    Ok((messages, last_message_id))
}

impl HistoryIterator<'_> {
    async fn new<'a>(
        channel: &'a HistoryChannel<'a>,
        http: Arc<serenity::http::Http>,
        limit: u8,
        before: Option<u64>,
        after: Option<u64>,
        around: Option<u64>,
    ) -> HistoryIterator<'a> {
        let (messages_gotten, last_message_id) =
            match fill_messages(channel, http.clone(), None, limit, before, after, around).await {
                Ok((messages_gotten, last_message_id)) => (messages_gotten, last_message_id),
                Err(e) => {
                    warn!("failed to get messages: {:?}", e);
                    (Vec::new(), None)
                }
            };
        HistoryIterator {
            channel,
            http,
            last_message_id,
            limit,
            current: 0,
            before,
            after,
            around,
            messages: messages_gotten,
        }
    }
}

// #[async_trait]
impl<'a> Iterator for HistoryIterator<'a> {
    type Item = Message;

    async fn next(&mut self) -> Option<Message> {
        if self.limit < self.current {
            return None;
        }

        if self.messages.is_empty() {
            return None;
        }

        let message = self.messages.remove(0);
        self.current += 1;

        if self.messages.is_empty() && self.limit - self.current != 0 && self.limit != self.current
        {
            let (messages_gotten, last_message_id) = fill_messages(
                self.channel,
                self.http.clone(),
                self.last_message_id,
                self.limit - self.current,
                self.before,
                self.after,
                self.around,
            )
            .await
            .unwrap();
            self.messages = messages_gotten;
            self.last_message_id = last_message_id;
        }

        Some(message)
    }
}

/// Loads messages fom server onto the database
#[poise::command(slash_command, guild_only,
// required_permissions = "ADMINISTRATOR"
)]
pub async fn load_messages(
    ctx: Context<'_>,
    #[description = "Reset messages (default off)"] reset: Option<bool>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx.guild().unwrap().clone();

    if reset.unwrap_or(false) {
        if let Some(guild) = Guilds::find_by_id(guild.id.get() as i64)
            .one(&ctx.data().db)
            .await?
        {
            warn!("deleted guild {:?}", guild.snowflake);
            guild.delete(&ctx.data().db).await?;
        }
    }

    let http = ctx.serenity_context().http.clone();

    let timer = Instant::now();

    let mut messages: Vec<Message> = futures::future::join_all(
        guild
            .channels(&ctx.http())
            .await?
            .par_iter()
            .map(|(.., c)| HistoryChannel::Guild(c))
            .map(|c| (c, http.clone()))
            .map(async move |(channel, http)| {
                HistoryIterator::new(&channel, http, u8::MAX, None, None, None)
                    .await
                    .collect::<Vec<_>>()
                    .await
                    .clone()
            })
            .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .progress()
    .flatten()
    .filter(|message| !message.author.bot)
    .collect();

    messages.sort_unstable_by(|a, b| a.id.cmp(&b.id));

    let cache = ctx.serenity_context().cache.clone();
    let data = Arc::new(ctx.data());

    let mut guild_scores = HashMap::new();
    let mut guild_message_count = HashMap::new();

    let mut channel_scores = HashMap::new();
    let mut channel_message_count = HashMap::new();
    let channel_to_guild = Channels::find()
        .select_column(entity::channels::Column::Snowflake)
        .select_column(entity::channels::Column::Guild)
        .all(&data.db)
        .await?
        .iter()
        .map(|model| (model.snowflake as u64, model.guild as u64))
        .collect::<HashMap<u64, u64>>();

    let mut user_scores = HashMap::new();
    let mut user_message_count = HashMap::new();

    for message in messages.clone().into_iter().progress() {
        let score = score_message(&message, &data.db).await;

        match handle_message(
            score,
            &http,
            &data,
            &message,
            Some(guild.id),
            &cache,
            false,
            &data.guild_in_db,
            &data.channel_in_db,
            &data.user_in_db,
        )
        .await
        {
            Ok(_) => {
                match message.guild_id {
                    Some(guild_id) => {
                        *guild_scores.entry(guild_id).or_insert(0.0) += score;
                        *guild_message_count.entry(guild_id).or_insert(0) += 1;
                    }
                    None => match channel_to_guild.get(&message.channel_id.get()) {
                        Some(guild_id) => {
                            *guild_scores.entry(GuildId::new(*guild_id)).or_insert(0.0) += score;
                            *guild_message_count
                                .entry(GuildId::new(*guild_id))
                                .or_insert(0) += 1;
                        }
                        None => {
                            *guild_scores
                                .entry(GuildId::new(guild.id.get()))
                                .or_insert(0.0) += score;
                            *guild_message_count
                                .entry(GuildId::new(guild.id.get()))
                                .or_insert(0) += 1;
                        }
                    },
                };

                *channel_scores.entry(message.channel_id).or_insert(0.0) += score;
                *channel_message_count.entry(message.channel_id).or_insert(0) += 1;

                *user_scores.entry(message.author.id).or_insert(0.0) += score;
                *user_message_count.entry(message.author.id).or_insert(0) += 1;
            }
            Err(e) => {
                warn!("failed to handle message: {:?} (as long as you dont see a billion of these messages you are probably fine)", e);
            }
        }
    }

    let guild_score_rs = futures::future::join_all(
        guild_scores
            .iter()
            .map(|(id, score)| (id, score, data.clone()))
            .map(async move |(guild_id, score, data)| {
                let mut a_guild = Guilds::find_by_id(guild_id.get() as i64)
                    .one(&data.db)
                    .await
                    .unwrap()
                    .unwrap()
                    .into_active_model();
                a_guild.score = Set(*score + a_guild.score.unwrap());

                a_guild.update(&data.db).await.unwrap();

                Ok::<(), Error>(())
            }),
    )
    .await;

    for guild_r in guild_score_rs {
        guild_r?;
    }

    let guild_count_rs = futures::future::join_all(
        guild_message_count
            .iter()
            .map(|(id, count)| (id, count, data.clone()))
            .map(async move |(guild_id, count, data)| {
                let mut a_guild = Guilds::find_by_id(guild_id.get() as i64)
                    .one(&data.db)
                    .await
                    .unwrap()
                    .unwrap()
                    .into_active_model();
                a_guild.message_count = Set(*count + a_guild.message_count.unwrap());

                a_guild.update(&data.db).await.unwrap();

                Ok::<(), Error>(())
            }),
    )
    .await;

    for guild_r in guild_count_rs {
        guild_r?;
    }

    let channel_rs = futures::future::join_all(
        channel_scores
            .iter()
            .map(|(id, score)| (id, score, data.clone()))
            .map(async move |(channel_id, score, data)| {
                let mut a_channel = entity::channels::Entity::find_by_id(channel_id.get() as i64)
                    .one(&data.db)
                    .await
                    .unwrap()
                    .unwrap()
                    .into_active_model();
                a_channel.score = Set(*score + a_channel.score.unwrap());

                a_channel.update(&data.db).await.unwrap();

                Ok::<(), Error>(())
            })
            .collect::<Vec<_>>(),
    )
    .await;

    for channel_r in channel_rs {
        channel_r?;
    }

    let channel_count_rs = futures::future::join_all(
        channel_message_count
            .iter()
            .map(|(id, count)| (id, count, data.clone()))
            .map(async move |(channel_id, count, data)| {
                let mut a_channel = entity::channels::Entity::find_by_id(channel_id.get() as i64)
                    .one(&data.db)
                    .await
                    .unwrap()
                    .unwrap()
                    .into_active_model();
                a_channel.message_count = Set(*count + a_channel.message_count.unwrap());

                a_channel.update(&data.db).await.unwrap();

                Ok::<(), Error>(())
            })
            .collect::<Vec<_>>(),
    )
    .await;

    for channel_r in channel_count_rs {
        channel_r?;
    }

    let user_rs = futures::future::join_all(
        user_scores
            .iter()
            .map(|(id, score)| (id, score, data.clone()))
            .map(async move |(user_id, score, data)| {
                loop {
                    match entity::users::Entity::find_by_id(user_id.get() as i64)
                        .one(&data.db)
                        .await
                    {
                        Ok(Some(user)) => {
                            let mut a_user = user.into_active_model();
                            a_user.score = Set(*score + a_user.score.unwrap());

                            a_user.update(&data.db).await.unwrap();

                            break;
                        }
                        Err(_) => {}
                        _ => {}
                    }
                }

                Ok::<(), Error>(())
            })
            .collect::<Vec<_>>(),
    )
    .await;

    for user_r in user_rs {
        user_r?;
    }

    let user_count_rs = futures::future::join_all(
        user_message_count
            .iter()
            .map(|(id, count)| (id, count, data.clone()))
            .map(async move |(user_id, count, data)| {
                loop {
                    match entity::users::Entity::find_by_id(user_id.get() as i64)
                        .one(&data.db)
                        .await
                    {
                        Ok(Some(user)) => {
                            let mut a_user = user.into_active_model();
                            a_user.message_count = Set(*count + a_user.message_count.unwrap());

                            a_user.update(&data.db).await.unwrap();

                            break;
                        }
                        Err(_) => {}
                        _ => {}
                    }
                }
                Ok::<(), Error>(())
            })
            .collect::<Vec<_>>(),
    )
    .await;

    for user_r in user_count_rs {
        user_r?;
    }

    // pb.finish_with_message("done");

    ctx.reply(format!(
        "got {} messages in {:?}",
        messages.len(),
        timer.elapsed()
    ))
    .await?;
    Ok(())
}
