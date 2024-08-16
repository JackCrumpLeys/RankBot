use crate::{Context, Data, Error};
use async_iterator::Iterator;
use indicatif::ProgressIterator;
use log::{debug, warn};
use migration::FromValueTuple;
use num_format::WriteFormatted;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseBackend, EntityTrait, IntoActiveModel, ModelTrait};
use serenity::all::{CacheHttp, Channel, ChannelId, UserId};
use serenity::builder::GetMessages;
use serenity::model::channel::GuildChannel;
use serenity::model::id::MessageId;
use serenity::model::prelude::Message;
use std::collections::HashMap;
use std::fmt::Write;
use std::io::Read;
use std::iter::{IntoIterator as StdIntoIterator, Iterator as stdIter};
use std::sync::Arc;

use crate::handlers::message::handle_message;
use crate::message_analyzer::score_message;
use entity::prelude::Guilds;
use tokio::time::Instant;

struct HistoryIterator<'a> {
    channel: &'a GuildChannel,
    http: Arc<serenity::http::Http>,
    limit: u32,
    current: u32,
    before: Option<u64>,
    after: Option<u64>,
    around: Option<u64>,
    messages: Vec<Message>,
}

async fn fill_messages(
    channel: &GuildChannel,
    http: Arc<serenity::http::Http>,
    last_message_id: Option<MessageId>,
    limit: u8,
    before: Option<u64>,
    after: Option<u64>,
    around: Option<u64>,
) -> Result<Vec<Message>, Error> {
    let messages_gotten = {
        let mut config = GetMessages::default();
        if let Some(last_message_id) = last_message_id {
            config = config.before(last_message_id);
            // println!(
            //     "selecting before {} on {} ({})",
            //     last_message_id, channel.id, channel.name
            // )
        } else if let Some(before) = before {
            config = config.before(before);
        }

        debug!(
            "selecting before {:?} on {} ({})",
            last_message_id, channel.id, channel.name
        );
        if let Some(after) = after {
            config = config.after(after);
        }

        if let Some(around) = around {
            config = config.around(around);
        }

        config = config.limit(limit);
        channel.messages(&http, config).await?
    };

    debug!(
        "got {} messages before {:?} on {} ({})",
        messages_gotten.len(),
        last_message_id,
        channel.id,
        channel.name,
    );
    // messages_gotten.reverse();
    Ok(messages_gotten)
}

impl HistoryIterator<'_> {
    async fn new<'a>(
        channel: &'a GuildChannel,
        http: Arc<serenity::http::Http>,
        limit: u32,
        before: Option<u64>,
        after: Option<u64>,
        around: Option<u64>,
    ) -> HistoryIterator<'a> {
        let messages_gotten = match fill_messages(
            channel,
            http.clone(),
            None,
            limit.min(100) as u8,
            before,
            after,
            around,
        )
        .await
        {
            Ok(messages_gotten) => messages_gotten,
            Err(e) => {
                warn!("failed to get messages: {:?}", e);
                Vec::new()
            }
        };
        HistoryIterator {
            channel,
            http,
            limit,
            current: 0,
            before,
            after,
            around,
            messages: messages_gotten,
        }
    }
}

impl<'a> async_iterator::Iterator for HistoryIterator<'a> {
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

        if self.messages.is_empty() && self.limit - self.current != 0 {
            debug!("{}", self.limit - self.current);
            let messages_gotten = fill_messages(
                self.channel,
                self.http.clone(),
                Some(message.id),
                (self.limit - self.current).min(100) as u8,
                self.before,
                self.after,
                self.around,
            )
            .await
            .unwrap();
            self.messages = messages_gotten;
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

    let channels = guild.channels(&ctx.http()).await?;
    let channel_map = channels.par_iter().map(|(.., c)| c);
    let mut messages: Vec<Message> = futures::future::join_all(
        channel_map
            .clone()
            .map(|c| (c, http.clone()))
            .map(async move |(channel, http)| {
                HistoryIterator::new(channel, http, u32::MAX, None, None, None)
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

    let mut message_log_file = std::fs::File::create("messages_recall.txt")?;
    // let message_json_file = std::fs::File::create("messages_recall.json")?;

    messages.sort_unstable_by(|a, b| a.id.cmp(&b.id));

    let cache = ctx.serenity_context().cache.clone();
    let data = Arc::new(ctx.data());

    let mut guild_score = 0.;
    let mut guild_message_count = 0;

    let mut channel_scores = HashMap::new();
    let mut channel_message_count = HashMap::new();

    let mut user_scores = HashMap::new();
    let mut user_message_count = HashMap::new();

    let mut message_log = String::new();

    let mut channel_id_name_map = HashMap::new();
    for channel in channel_map.collect::<Vec<&GuildChannel>>() {
        channel_id_name_map.insert(channel.id, channel.name.clone());
    }

    for message in messages.into_iter().progress() {
        message_log.push_str(
            format!(
                "{} [#{}] [{}] {}\n",
                message.timestamp.format("[%d-%m-%Y][%H:%M:%S]"),
                channel_id_name_map.get(&message.channel_id).unwrap(),
                message.author.name,
                message.content
            )
            .as_str(),
        );

        let score;
        {
            let mut last_five = data.last_five_map.write().await;

            let last_five = last_five.entry(message.author.id.clone()).or_insert(vec![]);

            score = score_message(&message, &last_five).await;

            last_five.push(message.content.clone());

            if last_five.len() == 6 {
                last_five.remove(0);
            }
            debug_assert!(last_five.len() < 6);
        }

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
                guild_score += score;
                guild_message_count += 1;

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

    std::io::Write::write(&mut message_log_file, message_log.as_str().as_bytes());

    let mut a_guild = Guilds::find_by_id(guild.id.get() as i64)
        .one(&data.db)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    a_guild.score = Set(guild_score + a_guild.score.unwrap());
    a_guild.message_count = Set(guild_message_count + a_guild.message_count.unwrap());

    a_guild.update(&data.db).await?;

    let channel_rs = futures::future::join_all(
        channel_scores
            .iter()
            .map(|(id, score)| (id, score, data.clone()))
            .map(
                async move |(channel_id, score, data): (&ChannelId, &f32, Arc<&Data>)| {
                    let mut a_channel =
                        entity::channels::Entity::find_by_id(channel_id.get() as i64)
                            .one(&data.db)
                            .await
                            .unwrap()
                            .unwrap()
                            .into_active_model();
                    a_channel.score = Set(*score + a_channel.score.unwrap());

                    a_channel.update(&data.db).await.unwrap();

                    Ok::<(), Error>(())
                },
            )
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
            .map(
                async move |(channel_id, count, data): (&ChannelId, &i32, Arc<&Data>)| {
                    let mut a_channel =
                        entity::channels::Entity::find_by_id(channel_id.get() as i64)
                            .one(&data.db)
                            .await
                            .unwrap()
                            .unwrap()
                            .into_active_model();
                    a_channel.message_count = Set(*count + a_channel.message_count.unwrap());

                    a_channel.update(&data.db).await.unwrap();

                    Ok::<(), Error>(())
                },
            )
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
            .map(
                async move |(user_id, score, data): (&UserId, &f32, Arc<&Data>)| {
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
                },
            )
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
            .map(
                async move |(user_id, count, data): (&UserId, &i32, Arc<&Data>)| {
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
                },
            )
            .collect::<Vec<_>>(),
    )
    .await;

    for user_r in user_count_rs {
        user_r?;
    }

    // pb.finish_with_message("done");

    ctx.reply(format!(
        "got {} messages in {:?}",
        guild_message_count,
        timer.elapsed()
    ))
    .await?;
    Ok(())
}
