use crate::scores::UserScore;
use crate::Context;
use crate::Error;
use chrono::Timelike;
use entity::prelude::{Channels, Messages, Users};
use entity::users::Model;
use num_format::Locale::en;
use num_format::ToFormattedString;
use poise::ReplyHandle;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;
use sea_orm::{EntityTrait, QueryOrder};
use serenity::model::channel::Channel;
use serenity::model::prelude::User;
use serenity::prelude::Mentionable;
use std::collections::{BTreeMap, HashMap};

struct StatMessage<'a> {
    title: String,
    fields: BTreeMap<String, Option<String>>,
    message: ReplyHandle<'a>,
    ctx: &'a Context<'a>,
}

impl StatMessage<'_> {
    async fn new<'a>(
        title: impl ToString,
        ctx: &'a Context<'a>,
        fields: Vec<impl ToString>,
    ) -> Result<StatMessage<'a>, Error> {
        Ok(StatMessage {
            title: title.to_string(),
            fields: fields
                .iter()
                .map(|field| (field.to_string(), None))
                .collect(),
            message: ctx
                .send(|m| {
                    m.embed(|e| {
                        e.title(title);

                        for field in fields {
                            e.field(
                                field.to_string(),
                                "<a:recall_loading:1163546685994188934>",
                                true,
                            );
                        }

                        e.colour(0x00ff00)
                    })
                })
                .await?,
            ctx,
        })
    }

    async fn set(
        &mut self,
        name: impl ToString,
        value: Option<impl ToString>,
    ) -> Result<(), Error> {
        self.fields
            .insert(name.to_string(), value.map(|v| v.to_string()));
        self.edit().await?;

        Ok(())
    }

    fn get_progress(&self) -> f32 {
        let mut progress = 0.0;
        for (_, value) in self.fields.iter() {
            if value.is_some() {
                progress += 1.0;
            }
        }

        progress / self.fields.len() as f32
    }

    async fn edit(&self) -> Result<(), Error> {
        let fields = self.fields.clone();
        self.message
            .edit(self.ctx.clone(), |m| {
                m.embed(|e| {
                    e.title(&self.title);

                    for (name, value) in fields {
                        e.field(
                            name,
                            value.unwrap_or("<a:recall_loading:1163546685994188934>".to_string()),
                            true,
                        );
                    }

                    if self.get_progress() == 1.0 {
                        e.footer(|f| {
                            f.icon_url(
                                "https://cdn.discordapp.com/emojis/1163591120840831046.gif?v=1",
                            );
                            f.text("Finished loading stats")
                        })
                    } else {
                        e.footer(|f| {
                            f.icon_url(
                                "https://cdn.discordapp.com/emojis/1163546685994188934.gif?v=1",
                            );
                            f.text(format!(
                                "Loading stats... {:.2}%",
                                self.get_progress() * 100.0
                            ))
                        })
                    };
                    e.colour(0x00ff00)
                })
            })
            .await?;
        Ok(())
    }
}

#[poise::command(slash_command, guild_only)]
pub async fn stats(
    ctx: Context<'_>,
    #[description = "User (defualt: you)"] user: Option<User>,
) -> Result<(), Error> {
    let db = &ctx.data().db;
    let user = match user {
        Some(ref user) => user,
        None => ctx.author(),
    };

    let guild_id = ctx.guild_id().unwrap();

    let user = match Users::find_by_id(user.id.0 as i64).one(db).await? {
        None => {
            ctx.send(|m| {
                m.embed(|e| {
                    e.title("User not found");
                    e.description(format!(
                        "User {} not found in database (Try saying something)",
                        user.tag()
                    ));
                    e.colour(0xff0000)
                })
            })
            .await?;

            return Ok(());
        }
        Some(user) => user,
    };
    let mut msg = StatMessage::new(
        format!("Stats for {}", user.name),
        &ctx,
        vec![
            "Score",
            "Score - week",
            "Score - month",
            "Score - year",
            "Rank",
            "Rank - week",
            "Rank - month",
            "Rank - year",
            "Best Channel",
            "Best Channel - week",
            "Best Channel - month",
            "Best Channel - year",
            "Messages",
            "XP summary",
            "Average score for messages",
            "Average score for messages - rank",
            "'Best' message",
            "Average post length",
            "3 most common uncommon words",
        ],
    )
    .await?;

    msg.set("Score", Some(user.score)).await?;
    msg.set(
        "Messages",
        Some(user.message_count.to_formatted_string(&en)),
    )
    .await?;
    msg.set(
        "XP summary",
        Some(UserScore::new(user.score).display_score()),
    )
    .await?;

    let last_week = chrono::Utc::now() - chrono::Duration::weeks(1);

    let last_week_of_messages = Messages::find()
        .filter(entity::messages::Column::User.eq(user.snowflake))
        .filter(entity::messages::Column::Timestamp.gt(last_week))
        .all(db)
        .await?;

    msg.set(
        "Score - week",
        Some(
            last_week_of_messages
                .iter()
                .map(|m| m.score)
                .sum::<f32>()
                .to_string(),
        ),
    )
    .await?;

    let last_month = chrono::Utc::now() - chrono::Duration::days(30);

    let last_month_of_messages = Messages::find()
        .filter(entity::messages::Column::User.eq(user.snowflake))
        .filter(entity::messages::Column::Timestamp.gt(last_month))
        .all(db)
        .await?;

    msg.set(
        "Score - month",
        Some(last_month_of_messages.iter().map(|m| m.score).sum::<f32>()),
    )
    .await?;

    let last_year = chrono::Utc::now() - chrono::Duration::days(365);

    let last_year_of_messages = Messages::find()
        .filter(entity::messages::Column::User.eq(user.snowflake))
        .filter(entity::messages::Column::Timestamp.gt(last_year))
        .all(db)
        .await?;

    msg.set(
        "Score - year",
        Some(last_year_of_messages.iter().map(|m| m.score).sum::<f32>()),
    )
    .await?;

    let mut users = Users::find()
        .order_by_desc(entity::users::Column::Score)
        .all(db)
        .await?;

    msg.set(
        "Rank",
        Some(
            (users
                .iter()
                .position(|u| u.snowflake == user.snowflake)
                .unwrap()
                + 1)
            .to_string(),
        ),
    )
    .await?;

    let last_week_of_messages = Messages::find()
        .filter(entity::messages::Column::Timestamp.gt(last_week))
        .all(db)
        .await?;

    /// use last week of messages
    let last_week_ranking = users
        .iter()
        .map(|u| {
            (
                u.snowflake,
                last_week_of_messages
                    .iter()
                    .filter(|m| m.user == u.snowflake)
                    .map(|m| m.score)
                    .sum::<f32>(),
            )
        })
        .collect::<HashMap<i64, f32>>();

    users.sort_by(|a, b| {
        last_week_ranking
            .get(&b.snowflake)
            .unwrap()
            .partial_cmp(last_week_ranking.get(&a.snowflake).unwrap())
            .unwrap()
    });

    msg.set(
        "Rank - week",
        Some(
            (users
                .iter()
                .position(|u| u.snowflake == user.snowflake)
                .unwrap()
                + 1)
            .to_string(),
        ),
    )
    .await?;

    let last_month_of_messages = Messages::find()
        .filter(entity::messages::Column::Timestamp.gt(last_week))
        .all(db)
        .await?;

    /// use last month of messages
    let last_month_ranking = users
        .iter()
        .map(|u| {
            (
                u.snowflake,
                last_month_of_messages
                    .iter()
                    .filter(|m| m.user == u.snowflake)
                    .map(|m| m.score)
                    .sum::<f32>(),
            )
        })
        .collect::<HashMap<i64, f32>>();

    users.sort_by(|a, b| {
        last_month_ranking
            .get(&b.snowflake)
            .unwrap()
            .partial_cmp(last_month_ranking.get(&a.snowflake).unwrap())
            .unwrap()
    });

    msg.set(
        "Rank - month",
        Some(
            (users
                .iter()
                .position(|u| u.snowflake == user.snowflake)
                .unwrap()
                + 1)
            .to_string(),
        ),
    )
    .await?;

    let last_year_of_messages = Messages::find()
        .filter(entity::messages::Column::Timestamp.gt(last_week))
        .all(db)
        .await?;

    /// use last year of messages
    let last_year_ranking = users
        .iter()
        .map(|u| {
            (
                u.snowflake,
                last_year_of_messages
                    .iter()
                    .filter(|m| m.user == u.snowflake)
                    .map(|m| m.score)
                    .sum::<f32>(),
            )
        })
        .collect::<HashMap<i64, f32>>();

    users.sort_by(|a, b| {
        last_year_ranking
            .get(&b.snowflake)
            .unwrap()
            .partial_cmp(last_year_ranking.get(&a.snowflake).unwrap())
            .unwrap()
    });

    msg.set(
        "Rank - year",
        Some(
            (users
                .iter()
                .position(|u| u.snowflake == user.snowflake)
                .unwrap()
                + 1)
            .to_string(),
        ),
    )
    .await?;

    let mut channels = Channels::find()
        .filter(entity::channels::Column::Guild.eq(ctx.guild_id().unwrap().0 as i64))
        .order_by_desc(entity::channels::Column::Score)
        .all(db)
        .await?;

    let mut highest_score_channel: Option<(&entity::channels::Model, f32)> = None;
    let mut week_highest_score_channel: Option<(&entity::channels::Model, f32)> = None;
    let mut month_highest_score_channel: Option<(&entity::channels::Model, f32)> = None;
    let mut year_highest_score_channel: Option<(&entity::channels::Model, f32)> = None;

    for channel in channels.iter() {
        let messages = Messages::find()
            .filter(entity::messages::Column::User.eq(user.snowflake))
            .filter(entity::messages::Column::Channel.eq(channel.snowflake))
            .all(db)
            .await?;

        let score = messages.iter().map(|m| m.score).sum::<f32>();

        if highest_score_channel.is_none() || score > highest_score_channel.unwrap().1 {
            highest_score_channel = Some((channel, score));
        }

        let last_week_of_messages = Messages::find()
            .filter(entity::messages::Column::User.eq(user.snowflake))
            .filter(entity::messages::Column::Channel.eq(channel.snowflake))
            .filter(entity::messages::Column::Timestamp.gt(last_week))
            .all(db)
            .await?;

        let score = last_week_of_messages.iter().map(|m| m.score).sum::<f32>();

        if week_highest_score_channel.is_none() || score > week_highest_score_channel.unwrap().1 {
            week_highest_score_channel = Some((channel, score));
        }

        let last_month_of_messages = Messages::find()
            .filter(entity::messages::Column::User.eq(user.snowflake))
            .filter(entity::messages::Column::Channel.eq(channel.snowflake))
            .filter(entity::messages::Column::Timestamp.gt(last_month))
            .all(db)
            .await?;

        let score = last_month_of_messages.iter().map(|m| m.score).sum::<f32>();

        if month_highest_score_channel.is_none() || score > month_highest_score_channel.unwrap().1 {
            month_highest_score_channel = Some((channel, score));
        }

        let last_year_of_messages = Messages::find()
            .filter(entity::messages::Column::User.eq(user.snowflake))
            .filter(entity::messages::Column::Channel.eq(channel.snowflake))
            .filter(entity::messages::Column::Timestamp.gt(last_year))
            .all(db)
            .await?;

        let score = last_year_of_messages.iter().map(|m| m.score).sum::<f32>();

        if year_highest_score_channel.is_none() || score > year_highest_score_channel.unwrap().1 {
            year_highest_score_channel = Some((channel, score));
        }
    }

    if let Some((channel, score)) = highest_score_channel {
        msg.set(
            "Best Channel",
            Some(format!(
                "{} - {}",
                ctx.http()
                    .get_channel(channel.snowflake as u64)
                    .await?
                    .mention(),
                score
            )),
        )
        .await?;
    } else {
        msg.set("Best Channel", Some("None")).await?;
    }

    if let Some((channel, score)) = week_highest_score_channel {
        msg.set(
            "Best Channel - week",
            Some(format!(
                "{} - {}",
                ctx.http()
                    .get_channel(channel.snowflake as u64)
                    .await?
                    .mention(),
                score
            )),
        )
        .await?;
    } else {
        msg.set("Best Channel - week", Some("None")).await?;
    }

    if let Some((channel, score)) = month_highest_score_channel {
        msg.set(
            "Best Channel - month",
            Some(format!(
                "{} - {}",
                ctx.http()
                    .get_channel(channel.snowflake as u64)
                    .await?
                    .mention(),
                score
            )),
        )
        .await?;
    } else {
        msg.set("Best Channel - month", Some("None")).await?;
    }

    if let Some((channel, score)) = year_highest_score_channel {
        msg.set(
            "Best Channel - year",
            Some(format!(
                "{} - {}",
                ctx.http()
                    .get_channel(channel.snowflake as u64)
                    .await?
                    .mention(),
                score
            )),
        )
        .await?;
    } else {
        msg.set("Best Channel - year", Some("None")).await?;
    }

    // average score for messages

    let messages = Messages::find()
        .filter(entity::messages::Column::User.eq(user.snowflake))
        .all(db)
        .await?;

    let score = messages.iter().map(|m| m.score).sum::<f32>() / messages.len() as f32;

    msg.set("Average score for messages", Some(format!("{:.2}", score)))
        .await?;

    let mut users = Users::find()
        .order_by_desc(entity::users::Column::Score)
        .all(db)
        .await?;

    let mut ranking = HashMap::new();

    for user in users.iter() {
        let messages = Messages::find()
            .filter(entity::messages::Column::User.eq(user.snowflake))
            .all(db)
            .await?;

        let score = messages.iter().map(|m| m.score).sum::<f32>() / messages.len() as f32;

        ranking.insert(user.snowflake, score);
    }

    users.sort_by(|a, b| {
        ranking
            .get(&b.snowflake)
            .unwrap()
            .partial_cmp(ranking.get(&a.snowflake).unwrap())
            .unwrap()
    });

    msg.set(
        "Average score for messages - rank",
        Some(
            (users
                .iter()
                .position(|u| u.snowflake == user.snowflake)
                .unwrap()
                + 1),
        ),
    )
    .await?;

    // 'best' message

    let best_message = messages
        .iter()
        .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
        .unwrap();

    msg.set(
        "'Best' message",
        Some(format!(
            "https://discord.com/channels/{}/{}/{} - {}",
            guild_id.0, best_message.channel, best_message.snowflake, best_message.score
        )),
    )
    .await?;

    let average_post_length =
        messages.iter().map(|m| m.content.len()).sum::<usize>() / messages.len();

    msg.set(
        "Average post length",
        Some(format!("{}", average_post_length)),
    )
    .await?;

    let mut words = HashMap::new();

    for message in messages.iter() {
        for word in message.content.split(" ") {
            if word.len() > 8 {
                if !word.starts_with("<@") && !word.ends_with(">") {
                    if !ctx.data().common_words.contains(word) {
                        let word = word.to_lowercase();
                        *words.entry(word).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut words = words.into_iter().collect::<Vec<(String, usize)>>();

    words.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    msg.set(
        "3 most common uncommon words",
        Some(format!(
            "{} - {} times\n{} - {} times\n{} - {} times",
            words[0].0, words[0].1, words[1].0, words[1].1, words[2].0, words[2].1,
        )),
    )
    .await?;

    Ok(())
}
