use super::bot::Context;
use mljcl::history::numscrobbles_async;
use mljcl::{range::Range, MalojaCredentials};
use poise::CreateReply;
use reqwest::Client;
use serenity::all::{CreateEmbed, CreateMessage, Message};

pub async fn artistscrobbles_cmd(
    client: Client,
    creds: Option<MalojaCredentials>,
    msg: Option<Message>,
    ctx: Context<'_>,
    arg: String,
) {
    if let Some(creds) = creds {
        let all_time_scrobbles = numscrobbles_async(
            Some(arg.clone()),
            Range::AllTime,
            creds.clone(),
            client.clone(),
        )
        .await
        .unwrap();
        let embed = CreateEmbed::new()
            .title(format!("{}'s scrobbles for {}", ctx.author().name, arg))
            .field("All time", all_time_scrobbles.to_string(), false);
        match msg {
            Some(msg) => {
                msg.channel_id
                    .send_message(ctx, CreateMessage::new().embed(embed))
                    .await
                    .unwrap();
            }
            None => {
                ctx.send(CreateReply::default().embed(embed)).await.unwrap();
            }
        }
    }
}

pub async fn scrobbles_cmd(
    client: Client,
    creds: Option<MalojaCredentials>,
    msg: Option<Message>,
    ctx: Context<'_>,
) {
    if let Some(creds) = creds {
        let all_time_scrobbles =
            numscrobbles_async(None, Range::AllTime, creds.clone(), client.clone())
                .await
                .unwrap();
        let this_year_scrobbles = numscrobbles_async(
            None,
            Range::In("thisyear".to_string()),
            creds,
            client.clone(),
        )
        .await
        .unwrap();
        let embed = CreateEmbed::new()
            .title(format!("{}'s scrobbles", ctx.author().name))
            .field("All time", all_time_scrobbles.to_string(), false)
            .field("This year", this_year_scrobbles.to_string(), false);
        match msg {
            Some(msg) => {
                msg.channel_id
                    .send_message(ctx, CreateMessage::new().embed(embed))
                    .await
                    .unwrap();
            }
            None => {
                ctx.send(CreateReply::default().embed(embed)).await.unwrap();
            }
        }
    };
}
