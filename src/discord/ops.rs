use mljcl::history::numscrobbles_async;
use mljcl::{range::Range, MalojaCredentials};
use reqwest::Client;
use serenity::all::{Context, CreateEmbed, CreateMessage, Message};

pub async fn artistscrobbles_cmd(
    msg: Message,
    client: Client,
    creds: Option<MalojaCredentials>,
    ctx: Context,
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
        msg.channel_id
            .send_message(
                ctx,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title(format!("{}'s scrobbles for {}", msg.author.name, arg))
                        .field("All time", all_time_scrobbles.to_string(), false),
                ),
            )
            .await
            .unwrap();
    }
}

pub async fn scrobbles_cmd(
    msg: Message,
    client: Client,
    creds: Option<MalojaCredentials>,
    ctx: Context,
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
        msg.channel_id
            .send_message(
                ctx,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title(format!("{}'s scrobbles", msg.author.name))
                        .field("All time", all_time_scrobbles.to_string(), false)
                        .field("This year", this_year_scrobbles.to_string(), false),
                ),
            )
            .await
            .unwrap();
    };
}
