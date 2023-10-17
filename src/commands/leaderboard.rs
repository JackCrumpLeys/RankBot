use crate::{Context, Error};
use entity::prelude::Users;
use sea_orm::{PaginatorTrait, QueryFilter};
use sea_orm::{ColumnTrait, EntityTrait, QueryOrder, QuerySelect};
use crate::scores::UserScore;

#[poise::command(slash_command, guild_only)]
pub async fn leaderboard(ctx: Context<'_>, #[description = "Page (default 1)"] page: Option<u16>) -> Result<(), Error> {
    let db = ctx.data();

    let guild_id = ctx.guild_id().unwrap();

    // users in the guild ordered by score
    let users = Users::find()
        .order_by_desc(entity::users::Column::Score)
        .filter(entity::users::Column::Guild.eq(guild_id.0 as i64))
        .paginate(&db.db,10)
        .fetch_page(page.unwrap_or(0) as u64)
        .await?;

    // use embeds to make it look nice
    ctx.send(|m| {
        m.embed(|e| {
            e.title(format!("Leaderboard page: {}", page.unwrap_or(1)));

            for user in users {
                e.field(user.name, UserScore::new(user.score).display_score(), false);
            }

            e.colour(0x00ff00)
        })
    })
    .await?;

    Ok(())
}
