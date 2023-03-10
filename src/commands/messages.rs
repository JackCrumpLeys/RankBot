use crate::handlers::message::handle_message;
use crate::{Context, Error};
use futures::future;
use std::collections::HashMap;

use log::{debug, trace};
use serenity::http::CacheHttp;

use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::time::Instant;

/// Loads messages fom server onto the database
#[poise::command(slash_command)]
pub async fn load_messages(
    ctx: Context<'_>,
    #[description = "Reset messages (default off)"] reset: Option<bool>,
) -> Result<(), Error> {
    let timer = Instant::now();
    let _reset = reset.unwrap_or(false);

    let guild = ctx.guild().unwrap();

    let channels = guild.channels(&ctx.http()).await?;
    let http = ctx.serenity_context().http.clone();

    ctx.defer().await?;

    let messages = Arc::new(RwLock::new(HashMap::new()));

    let channel_tasks: Vec<_> = channels
        .values()
        .map(|v| Arc::new(v.clone()))
        .map(|channel| {
            let http = http.clone();
            let messages = messages.clone();
            tokio::spawn(async move {
                let channel = channel.clone();
                for message in channel
                    .messages(&http, |retriever| retriever.limit(u64::MAX))
                    .await?
                    .into_iter()
                {
                    trace!("Added message: {}", message.id.clone());
                    messages.write().await.insert(message.id, message);
                }
                let mut last_message_count = messages.read().await.len();
                while last_message_count != 0 && last_message_count > 0 {
                    let last_message = messages.read().await;
                    let last_message = last_message.iter().last().unwrap();
                    for message in channel
                        .messages(&http, |retriever| {
                            retriever.before(last_message.0).limit(u64::MAX)
                        })
                        .await?
                        .into_iter()
                    {
                        trace!("Added message: {}", message.id.clone());
                        messages.write().await.insert(message.id, message);
                    }

                    last_message_count = messages.read().await.len();
                }
                debug!(
                    "Loaded {} messages from {}",
                    messages.read().await.len(),
                    channel.name
                );
                Ok::<_, Error>(())
            })
        })
        .collect();

    let _message = future::join_all(channel_tasks).await;

    // message
    //     .iter()
    //     .for_each(|m| match m {
    //         Ok(m) => match m {
    //             Ok(m) => trace!("Loaded message "),
    //             Err(e) => {
    //                 debug!("Error: {:?}", e);
    //             }
    //         },
    //         Err(e) => {
    //             debug!("Error loading messages: {}", e);
    //         }
    //     });
    let cache = ctx.serenity_context().cache.clone();
    let http = ctx.serenity_context().http.clone();
    let data = ctx.data().clone();
    // let pb = Arc::new(ProgressBar::new(messages_len as u64).clone());

    let message_tasks = messages.read().await;

    let message_tasks: Vec<_> = message_tasks
        .clone()
        .into_values()
        // .map(|m| m.1)
        .map(|message| {
            let http = http.clone();
            let cache = cache.clone();
            let data = data.clone();
            // let pb = pb.clone();
            let messages = messages.clone();
            tokio::spawn(async move {
                handle_message(
                    &http,
                    &data,
                    &message,
                    Some(guild.id),
                    &cache,
                    false,
                    messages,
                )
                .await?;
                // pb.inc(1);

                Ok::<(), Error>(())
            })
        })
        .collect();
    // pb.finish_with_message("done");
    future::join_all(message_tasks).await;

    debug!("load_messages took {:?}", timer.elapsed());
    let messages_len = messages.read().await.len();
    ctx.send(|builder| {
        builder.content(format!(
            "Loaded {} messages from {} channels in {:?}",
            messages_len,
            channels.len(),
            timer.elapsed()
        ))
    })
    .await?;
    Ok(())
}
