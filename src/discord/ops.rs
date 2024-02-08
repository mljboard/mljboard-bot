use super::bot::{Context, MljboardUser};
use crate::discord::lastfm::get_lastfm_user;
use crate::discord::lastfm::LfmRange;
use image::ImageBuffer;
use image::RgbaImage;
use mljcl::history::numscrobbles_async;
use mljcl::range::Range as MljRange;
use poise::CreateReply;
use reqwest::Client;
use serenity::all::{CreateAttachment, CreateEmbed, CreateMessage, Message};
use std::io::Cursor;
use std::time::SystemTime;

use image::imageops::FilterType;
use image::DynamicImage;

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

pub async fn grid_cmd(
    client: Client,
    user: Option<MljboardUser>,
    _msg: Option<Message>,
    ctx: Context<'_>,
    square_size: usize,
    range: mljcl::range::Range,
) {
    let album_count = square_size.pow(2);

    if let Some(user) = user {
        match user {
            MljboardUser::MalojaUser(user) => {
                ctx.defer().await.unwrap(); // Apparently needed for size > 1 because requests simply take too long

                let albums_ranked =
                    mljcl::charts::charts_albums_async(range, None, user.clone(), client.clone())
                        .await
                        .map(|x| x.albums);
                if let Ok(mut albums_ranked) = albums_ranked {
                    albums_ranked.truncate(album_count);
                    let top_album_ids: Vec<String> = albums_ranked
                        .into_iter()
                        .map(|(album, _)| album.id)
                        .collect();

                    let image_width = 64;
                    let image_height = 64;

                    let mut grid_image: RgbaImage = ImageBuffer::new(
                        image_width * square_size as u32,
                        image_height * square_size as u32,
                    );

                    let mut x = 0;
                    let mut y = 0;

                    let mut image_count = 0;

                    for album_id in top_album_ids {
                        let album_art_bytes =
                            mljcl::art::album_art_async(album_id, user.clone(), client.clone())
                                .await
                                .unwrap();
                        if let Ok(img) = image::load_from_memory(&album_art_bytes) {
                            let mut image = DynamicImage::ImageRgba8(image::imageops::resize(
                                &img,
                                image_width,
                                image_height,
                                FilterType::CatmullRom,
                            ));
                            image::imageops::overlay(
                                &mut grid_image,
                                image.as_mut_rgba8().unwrap(),
                                x,
                                y,
                            );
                            image_count += 1;
                            x += 64;
                            if image_count >= square_size {
                                x = 0;
                                y += 64;
                                image_count = 0;
                            }
                        }
                    }

                    let mut grid_image_bytes: Vec<u8> = Vec::new();
                    grid_image
                        .write_to(
                            &mut Cursor::new(&mut grid_image_bytes),
                            image::ImageOutputFormat::Png,
                        )
                        .unwrap();
                    let attachment = CreateAttachment::bytes(grid_image_bytes, "grid.png");

                    let message = CreateReply::default()
                        .attachment(attachment)
                        .embed(CreateEmbed::new().attachment("grid.png"));

                    ctx.send(message).await.unwrap();
                } else {
                    ctx.reply("There was an error getting your album chart.")
                        .await
                        .unwrap();
                }
            }
            MljboardUser::LastFMUser(_) => {
                ctx.reply("Grids are not implemented for Last.FM users yet.")
                    .await
                    .unwrap();
            }
        }
    }
}
