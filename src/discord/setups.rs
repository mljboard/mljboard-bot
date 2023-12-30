use super::bot::Context;
use crate::db::postgres::*;
use crate::dm_channel;
use serenity::all::CreateMessage;
use sqlx::PgPool;

pub async fn hos_setup(ctx: Context<'_>, pool: &PgPool, formatted_user: String) {
    if let Some(dm_channel) = dm_channel!(ctx) {
        let mut match_found = false;

        let query = get_discord_pairing_code(pool, formatted_user.clone()).await;

        if !query.is_empty() {
            match_found = true;
        }

        if !match_found {
            let key = crate::generate_api_key();
            //TODO: check unique
            insert_discord_pairing_code(pool, formatted_user, key.clone()).await;
            dm_channel.send_message(ctx,
                    CreateMessage::new().content(format!("You have been assigned the pairing code `{}`. Make sure to pass this to your HOS client.", key))
                ).await.unwrap();
        } else {
            dm_channel
                    .send_message(ctx,
                        CreateMessage::new().content(
                            "You've already made a pairing code, or you have a website linked. Do `!reset` to revoke the code and/or remove the website.",
                        )
                    )
                    .await
                    .unwrap();
        }

        let _ = ctx.say("DMed you.").await;
    } else {
        let _ = ctx.say("Unable to create a DM channel with you.").await;
    }
}

pub async fn website_setup(ctx: Context<'_>, pool: &PgPool, formatted_user: String, arg: String) {
    let mut match_found = false;

    let query = get_websites(pool, formatted_user.clone()).await;
    if !query.is_empty() {
        match_found = true;
    }

    if match_found {
        ctx.say("You've already set a website. Do `!reset` to remove it.")
            .await
            .unwrap();
        return;
    }
    if !arg.is_empty() {
        if !(arg.starts_with("http://") || arg.starts_with("https://")) {
            ctx.say("Remember that your website has to start with `http://` or `https://`. Try again with \
                    one of those two, and keep in mind if you're using https you cannot use an invalid certificate.").await.unwrap();
            return;
        }
        ctx.say(format!("Setting your website to {}.", arg))
            .await
            .unwrap();

        insert_website(pool, formatted_user, arg).await;
    } else {
        ctx.say("No website provided.").await.unwrap();
    }
}

pub async fn lfm_setup(ctx: Context<'_>, pool: &PgPool, formatted_user: String, arg: String) {
    let mut match_found = false;

    let query: Vec<DiscordLastFMUser> = get_lastfm_username(pool, formatted_user.clone()).await;
    if !query.is_empty() {
        match_found = true;
    }

    if match_found {
        ctx.say("You've already set a Last.FM username. Do `!reset` to remove it.")
            .await
            .unwrap();
        return;
    }

    if !arg.is_empty() {
        if arg.len() >= 50 {
            ctx.say("Your Last.FM username is way too long.")
                .await
                .unwrap(); // as far as I know, the limit is 15 characters
            return;
        }
        if !arg.chars().all(char::is_alphanumeric) {
            ctx.say("Your provided Last.FM username is invalid.")
                .await
                .unwrap();
            return;
        }
        ctx.say(format!("Setting your Last.FM username to {}.", arg))
            .await
            .unwrap();

        insert_lastfm_user(pool, formatted_user, arg).await;
    } else {
        ctx.say("No website provided.").await.unwrap();
    }
}

pub async fn reset(ctx: Context<'_>, pool: &PgPool, formatted_user: String) {
    if let Some(dm_channel) = dm_channel!(ctx) {
        for row in get_websites(pool, formatted_user.clone()).await {
            if row.discord_username == Some(formatted_user.clone()) {
                dm_channel
                    .send_message(
                        ctx,
                        CreateMessage::new().content(format!(
                            "Removing your website `{}` from mljboard's database. \
                    Run `!site_setup` to assign yourself one.",
                            row.website.unwrap_or("[none]".to_string())
                        )),
                    )
                    .await
                    .unwrap();
            }
        }

        let query = delete_website(pool, formatted_user.clone()).await;

        if query >= 1 {
            dm_channel
                .send_message(
                    ctx,
                    CreateMessage::new().content(format!("Removed {} entries.", query)),
                )
                .await
                .unwrap();
        }

        let query = get_discord_pairing_code(pool, formatted_user.clone()).await;

        let mut affected: u16 = 0;

        for row in query {
            affected += 1;
            dm_channel
                .send_message(
                    ctx,
                    CreateMessage::new().content(format!(
                        "Removing your pairing code `{}` from mljboard's database. \
                    Run `!hos_setup` to be issued a new one.",
                        row.pairing_code.unwrap_or("[none]".to_string())
                    )),
                )
                .await
                .unwrap();
        }

        let query = delete_discord_pairing_code(pool, formatted_user.clone()).await;

        if query >= 1 {
            dm_channel
                .send_message(
                    ctx,
                    CreateMessage::new().content(format!("Removed {} entries.", query)),
                )
                .await
                .unwrap();
        }

        if affected == 0 {
            dm_channel
                .send_message(
                    ctx,
                    CreateMessage::new()
                        .content("We couldn't find any pairing codes that were yours."),
                )
                .await
                .unwrap();
        }

        let query = get_lastfm_username(pool, formatted_user.clone()).await;

        for row in query {
            affected += 1;
            dm_channel
                .send_message(
                    ctx,
                    CreateMessage::new().content(format!(
                        "Removing your Last.FM username `{}` from mljboard's database.",
                        row.lastfm_username.unwrap_or("[none]".to_string())
                    )),
                )
                .await
                .unwrap();
        }

        let query = delete_lastfm_user(pool, formatted_user.clone()).await;

        if query >= 1 {
            dm_channel
                .send_message(
                    ctx,
                    CreateMessage::new().content(format!("Removed {} entries.", query)),
                )
                .await
                .unwrap();
        }

        let _ = ctx.say("DMed you.").await;
    } else {
        let _ = ctx.say("Unable to create a DM channel with you.").await;
    }
}
