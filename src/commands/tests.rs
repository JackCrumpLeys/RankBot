use crate::{Context, Error};
use indicatif::ProgressBar;
use std::time::Duration;

/// a command to test progress bars
#[poise::command(slash_command)]
pub async fn test_progress_bar(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(|b| b.content("Starting progress bar")).await?;
    let pb = ProgressBar::new(100);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}",
            )?
            .progress_chars("##-"),
    );
    pb.set_message("Loading...");
    for i in 0..100 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        pb.inc(i);
    }
    pb.finish_with_message("Done!");
    Ok(())
}
