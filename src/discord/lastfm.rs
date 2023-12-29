use super::bot::Context;
use crate::lfm::*;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use lastfm::{client::RecentTracksFetcher, track::RecordedTrack};
use poise::{serenity_prelude::*, CreateReply, ReplyHandle};
use serenity::all::CreateEmbed;
use std::{future::IntoFuture, time::SystemTime};

const LOADING_GIF: &str = "https://media1.tenor.com/m/mRbYKHgYCOIAAAAC/loading-gif-loading.gif";

pub fn get_loading_message(username: String, loaded: Option<usize>) -> CreateReply {
    let text = format!(
        "Working on scrobbles for LastFM user {}... ({} loaded)",
        username.clone(),
        loaded.unwrap_or(0)
    );
    return CreateReply::default()
        .embed(CreateEmbed::new().title(text).image(LOADING_GIF))
        .components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
            "cancel",
        )
        .label("Cancel")])]);
}

pub async fn get_streams(
    recent_tracks: Result<RecentTracksFetcher, lastfm::errors::Error>,
    message: ReplyHandle<'_>,
    ctx: Context<'_>,
    username: String,
) -> Option<Vec<RecordedTrack>> {
    let recent_stream = recent_tracks.map(|x| x.into_stream());
    if let Ok(recent_stream) = recent_stream {
        pin_mut!(recent_stream);
        let mut tracks = vec![];
        while let Some(track) = recent_stream.next().await {
            tracks.push(track.unwrap());
            if tracks.len() % 1000 == 0 {
                message
                    .edit(
                        ctx.clone(),
                        get_loading_message(username.clone(), Some(tracks.len())),
                    )
                    .await
                    .expect("Error editing message");
            }
        }

        return Some(tracks);
    }
    None
}

pub async fn get_lastfm_user(
    ctx: Context<'_>,
    api: String,
    username: String,
    from: Option<i64>,
    to: Option<i64>,
) -> Option<Vec<RecordedTrack>> {
    // TODO: caching will skip this step or parts of it *if available*
    let message = ctx
        .send(get_loading_message(username.clone(), None))
        .await
        .unwrap();

    let user = get_lastfm_client(username.clone(), api.clone()).await;

    let recent_tracks = user.recent_tracks(from, to).await;
    let ret: Option<Vec<RecordedTrack>>;
    tokio::select! {
        stream_vec = get_streams(recent_tracks, message.clone(), ctx, username) => {
            ret = stream_vec;
        },
        _ = message.message().await.unwrap()
        .await_component_interaction(ctx)
        .author_id(ctx.author().id)
        .custom_ids(vec!["cancel".to_string()]).into_future() => {
            ret = None;
        }
    }

    // we don't unwrap here just in case the bot isn't able to delete its own messages
    // this is unlikely but there's no reason to crash the entire command for that
    let _ = message.delete(ctx.clone()).await;

    ret
}

pub async fn lfmuser_cmd(ctx: Context<'_>, api: Option<String>, arg: String) {
    match api {
        Some(lastfm_api) => {
            let now_secs = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|x| x.as_secs() as i64)
                .ok();
            let one_year_ago = now_secs.map(|x| x - 31_536_000);
            let tracks =
                get_lastfm_user(ctx.clone(), lastfm_api, arg.clone(), one_year_ago, now_secs).await;

            let trackcount = match tracks {
                Some(tracks) => tracks.len().to_string(),
                None => "[user not found, or cancel occurred]".to_string(),
            };

            ctx.send(
                CreateReply::default().embed(
                    CreateEmbed::new()
                        .title(format!("LastFM user {}'s scrobbles", arg.clone()))
                        .field("Within the past year", trackcount, false),
                ),
            )
            .await
            .unwrap();
        }
        None => {
            ctx.say("The bot owner has not set up a Last.FM API key.")
                .await
                .expect("Error sending Discord message");
        }
    }
}
