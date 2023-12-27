use crate::lfm::*;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use lastfm::track::RecordedTrack;
use serenity::all::{Context, CreateEmbed, CreateMessage, EditMessage, Message};
use std::time::SystemTime;

const LOADING_GIF: &str = "https://media1.tenor.com/m/mRbYKHgYCOIAAAAC/loading-gif-loading.gif";

pub async fn get_lastfm_user(
    ctx: Context,
    msg: Message,
    api: String,
    username: String,
    from: Option<i64>,
    to: Option<i64>,
) -> Option<Vec<RecordedTrack>> {
    let mut message = msg
        .channel_id
        .send_message(
            ctx.clone(),
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title(format!(
                        "Working on scrobbles for LastFM user {}... (0 loaded)",
                        username.clone()
                    ))
                    .image(LOADING_GIF),
            ),
        )
        .await
        .unwrap();

    let user = get_lastfm_client(username.clone(), api.clone()).await;

    let recent_stream = user.recent_tracks(from, to).await.map(|x| x.into_stream());
    let mut ret: Option<Vec<RecordedTrack>> = None;
    if let Ok(recent_stream) = recent_stream {
        pin_mut!(recent_stream);
        let mut tracks = vec![];
        while let Some(track) = recent_stream.next().await {
            tracks.push(track.unwrap());
            if tracks.len() % 1000 == 0 {
                message
                    .edit(
                        ctx.clone(),
                        EditMessage::new().embed(
                            CreateEmbed::new()
                                .title(format!(
                                    "Working on scrobbles for LastFM user {}... ({} loaded)",
                                    username.clone(),
                                    tracks.len()
                                ))
                                .image(LOADING_GIF),
                        ),
                    )
                    .await
                    .expect("Error editing message");
            }
        }

        ret = Some(tracks);
    }

    // we don't unwrap here just in case the bot isn't able to delete its own messages
    // this is unlikely but there's no reason to crash the entire command for that
    let _ = message.delete(ctx.clone()).await;

    ret
}

pub async fn lfmuser_cmd(ctx: Context, msg: Message, api: Option<String>, arg: String) {
    match api {
        Some(lastfm_api) => {
            let now_secs = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|x| x.as_secs() as i64)
                .ok();
            let one_year_ago = now_secs.map(|x| x - 31_536_000);
            let tracks = get_lastfm_user(
                ctx.clone(),
                msg.clone(),
                lastfm_api,
                arg.clone(),
                one_year_ago,
                now_secs,
            )
            .await;

            let trackcount = match tracks {
                Some(tracks) => tracks.len().to_string(),
                None => "[user not found]".to_string(),
            };

            msg.channel_id
                .send_message(
                    ctx,
                    CreateMessage::new().embed(
                        CreateEmbed::new()
                            .title(format!("LastFM user {}'s scrobbles", arg.clone()))
                            .field("Within the past year", trackcount, false),
                    ),
                )
                .await
                .unwrap();
        }
        None => {
            msg.reply(ctx, "The bot owner has not set up a Last.FM API key.")
                .await
                .expect("Error sending discord message");
        }
    }
}
