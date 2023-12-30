use super::bot::Context;
use crate::discord::bot::format_user;
use poise::serenity_prelude::*;

pub async fn try_dm_channel(author: User, ctx: Context<'_>) -> Option<PrivateChannel> {
    let dm_channel = author.create_dm_channel(ctx).await;
    match dm_channel {
        Ok(dm_channel) => Some(dm_channel),
        Err(err) => {
            let _ = ctx.say("Couldn't create a DM channel with you").await;
            log::error!(
                "Error creating a DM channel with {}, {}",
                format_user(author),
                err
            );
            None
        }
    }
}

#[macro_export]
macro_rules! dm_channel {
    ( $ctx:ident ) => {
        $crate::discord::macros::try_dm_channel($ctx.author().clone(), $ctx.clone()).await
    };
}
