use super::bot::{Context, MljboardUser};
use crate::discord::lastfm::get_lastfm_user;
use crate::discord::lastfm::LfmRange;
use mljcl::range::Range as MljRange;
use mljcl::history::numscrobbles_async;
use poise::CreateReply;
use reqwest::Client;
use serenity::all::{CreateEmbed, CreateMessage, Message};
use std::time::SystemTime;

pub enum GetScrobbleCountFailed {
    UserNotFound,
    CancelOccurred,
    MalojaError,
}

impl ToString for GetScrobbleCountFailed {
    fn to_string(&self) -> String {
        match self {
            GetScrobbleCountFailed::UserNotFound => "Last.FM user not found".to_string(),
            GetScrobbleCountFailed::CancelOccurred => "Cancel occurred".to_string(),
            GetScrobbleCountFailed::MalojaError => "Maloja error".to_string(),
        }
    }
}

pub async fn get_scrobble_count(
    client: Client,
    user: MljboardUser,
    _msg: Option<Message>,
    ctx: Context<'_>,
    artist: Option<String>,
    maloja_range: MljRange,
    lfm_range: LfmRange,
) -> Result<u64, GetScrobbleCountFailed> {
    match user {
        MljboardUser::LastFMUser(lfm) => {
            if !lfm_range.available() {
                // we shouldn't use the `lastfm` crate here since it only gets individual tracks (which takes a while), we need to use my lib to get user summary
                lfm_stats::user_get_info(
                    lfm.username,
                    ctx.data()
                        .lastfm_api
                        .clone()
                        .expect("No Last.FM API key passed"),
                )
                .await
                .map(|x| x.scrobbles)
                .map_err(|_| GetScrobbleCountFailed::UserNotFound)
            } else {
                let mut ret = 0;
                let tracks = get_lastfm_user(
                    ctx,
                    ctx.data()
                        .lastfm_api
                        .clone()
                        .expect("No Last.FM API key passed"),
                    lfm.username.clone(),
                    lfm_range,
                )
                .await;
                match tracks {
                    Some(tracks) => {
                        if let Some(artist) = artist {
                            for track in tracks {
                                if track.artist.name.to_uppercase() == artist.to_uppercase() {
                                    ret += 1;
                                }
                            }
                        } else {
                            ret = tracks.len();
                        }

                        Ok(ret.try_into().unwrap())
                    }
                    None => Err(GetScrobbleCountFailed::CancelOccurred),
                }
            }
        }
        MljboardUser::MalojaUser(creds) => {
            numscrobbles_async(artist, maloja_range, creds.clone(), client.clone())
                .await
                .map_err(|_| GetScrobbleCountFailed::MalojaError)
        }
    }
}

pub fn human_readable_result(result: Result<u64, GetScrobbleCountFailed>) -> String {
    match result {
        Ok(count) => count.to_string(),
        Err(error) => error.to_string(),
    }
}

pub async fn artistscrobbles_cmd(
    client: Client,
    user: Option<MljboardUser>,
    msg: Option<Message>,
    ctx: Context<'_>,
    arg: String,
) {
    if let Some(user) = user {
        let all_time_scrobbles: String = human_readable_result(
            get_scrobble_count(
                client,
                user,
                msg.clone(),
                ctx,
                Some(arg.clone()),
                MljRange::AllTime,
                LfmRange::new(None, None),
            )
            .await,
        );

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
    user: Option<MljboardUser>,
    msg: Option<Message>,
    ctx: Context<'_>,
) {
    let this_year_scrobbles: String; // if we fail to get scrobbles, we need an error message
    let all_time_scrobbles: String;
    if let Some(user) = user {
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|x| x.as_secs() as i64)
            .ok();
        let one_year_ago = now_secs.map(|x| x - 31_536_000);

        this_year_scrobbles = human_readable_result(
            get_scrobble_count(
                client.clone(),
                user.clone(),
                msg.clone(),
                ctx,
                None,
                MljRange::In("thisyear".to_string()),
                LfmRange::new(one_year_ago, now_secs),
            )
            .await,
        );
        all_time_scrobbles = human_readable_result(
            get_scrobble_count(
                client,
                user,
                msg.clone(),
                ctx,
                None,
                MljRange::AllTime,
                LfmRange::new(None, None),
            )
            .await,
        );

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
    }
}
