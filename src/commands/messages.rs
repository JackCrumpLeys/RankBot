use tokio::time::Instant;
use log::debug;
use futures::future;
use serenity::model::channel::Message;
use std::sync::Arc;
use indicatif::ProgressBar;
use serenity::http::CacheHttp;
use crate::{Context, Error};
use crate::handlers::message::handle_message;

/// Loads messages fom server onto the database
#[poise::command(slash_command)]
pub async fn load_messages(
    ctx: Context<'_>,
    #[description = "Reset messages (default off)"] reset: Option<bool>,
) -> Result<(), Error> {
    let timer = Instant::now();
    let reset = reset.unwrap_or(false);

    let guild = ctx.guild().unwrap();

    let channels = guild.channels(&ctx.http()).await?;
    let mut http = ctx.serenity_context().http.clone();

    ctx.defer().await?;

    let channel_tasks: Vec<_> = channels
        .iter()
        .map(|(k,v)| {
            v.clone()// get values
        })
        .map(move |channel| {
            let http = http.clone();
            tokio::spawn(async move {
                let channel = channel.clone();
                let mut messages = channel.messages(&http, |retriever| {
                    retriever.limit(u64::MAX)
                }).await?;
                let mut last_message_count = messages.len();
                while last_message_count % 100 == 0 && last_message_count > 0 {
                    let last_message = messages.last().unwrap();
                    messages.append(&mut channel.messages(&http, |retriever| {
                        retriever.before(last_message.id).limit(u64::MAX)
                    }).await?);
                    last_message_count = messages.len();
                }
                debug!("Loaded {} messages from {}", messages.len(), channel.name);
                Ok::<_, Error>(messages)
            })
        })
        .collect();

    let mut message = future::join_all(channel_tasks).await;


    let mut messages = message.iter().map(|m| {
        match m {
            Ok(m) => match m {
                Ok(m) => m.to_vec(),
                Err(e) => {
                    debug!("Error: {:?}", e);
                    Vec::new()
                }
            },
            Err(e) => {
                debug!("Error loading messages: {}", e);
                Vec::new()
            }
        }
    }).flatten().collect::<Vec<Message>>();

    let mut cache = ctx.serenity_context().cache.clone();
    let mut http = ctx.serenity_context().http.clone();
    let mut data = ctx.data().clone();
    let pb = Arc::new(ProgressBar::new(messages.len() as u64).clone());

    let message_tasks: Vec<_> =
        messages
        .iter()
        .map(|m| {
            m.clone()
        })
        .map(|message| {
            let http = http.clone();
            let cache = cache.clone();
            let data = data.clone();
            let pb = pb.clone();
            tokio::spawn(async move {
                handle_message(&http, &data, &message, Some(guild.id), &cache, false)
                    .await?;
                pb.inc(1);

                Ok::<(), Error>(())
            })
        })
        .collect();
    pb.finish_with_message("done");
    future::join_all(message_tasks).await;

    debug!("load_messages took {:?}", timer.elapsed());
    ctx.send(|builder| {
        builder.content(format!(
            "Loaded {} messages from {} channels in {:?}",
            messages.len(),
            channels.len(),
            timer.elapsed()
        ))
    }).await?;
    Ok(())
}
