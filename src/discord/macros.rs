use crate::discord::bot::format_user;
use serenity::model::channel::PrivateChannel;
use serenity::model::prelude::Message;
use serenity::model::user::User;
use serenity::prelude::*;

pub async fn try_dm_channel(
    author: User,
    original_message: Option<Message>,
    ctx: Context,
) -> Option<PrivateChannel> {
    let dm_channel = author.create_dm_channel(ctx.clone()).await;
    match dm_channel {
        Ok(dm_channel) => Some(dm_channel),
        Err(err) => {
            let _ = original_message?
                .reply_ping(ctx, "Couldn't create a DM channel with you")
                .await;
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
    ( $msg:ident, $ctx:ident ) => {
        $crate::discord::macros::try_dm_channel(
            $msg.clone().author,
            Some($msg.clone()),
            $ctx.clone(),
        )
        .await
    };
}
