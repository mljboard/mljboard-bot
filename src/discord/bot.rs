use crate::db::postgres::{get_discord_pairing_code, get_lastfm_username, get_websites};
use crate::hos::*;
use core::num::NonZeroU16;
use mljcl::MalojaCredentials;
use poise::serenity_prelude::*;
use sqlx::PgPool;
use std::result::Result;
use url::{ParseError, Url};

use super::lastfm::LastFMUser;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, BotData, Error>;

#[derive(Clone, Debug)]
pub struct BotData {
    pub pool: PgPool,
    pub hos_server_ip: String,
    pub hos_server_port: u16,
    pub hos_server_passwd: Option<String>,
    pub hos_server_https: bool,
    pub reqwest_client: reqwest::Client,
    pub lastfm_api: Option<String>,
}

#[derive(Clone, Debug)]
pub enum MljboardUser {
    MalojaUser(MalojaCredentials),
    LastFMUser(LastFMUser),
}

fn option_nonzerou16_to_u16(input: Option<NonZeroU16>) -> u16 {
    // we want to keep the `#0` after the user
    // it can't hurt
    match input {
        Some(output) => output.get(),
        None => 0,
    }
}

pub fn format_user(user: User) -> String {
    format!(
        "{}#{}",
        user.name,
        option_nonzerou16_to_u16(user.discriminator)
    )
}

impl BotData {
    pub async fn handle_hos_user(
        &self,
        formatted_user: String,
        ctx: Context<'_>,
    ) -> Option<MalojaCredentials> {
        let mut assigned_pairing_code: Option<String> = None;
        for result in get_discord_pairing_code(&self.pool, formatted_user).await {
            assigned_pairing_code = result.pairing_code;
        }
        match assigned_pairing_code {
            Some(pairing_code) => {
                let base = match self.hos_server_https {
                    true => "https://",
                    false => "http://",
                };
                let connections = get_hos_connections(
                    base.to_string(),
                    self.hos_server_ip.clone(),
                    self.hos_server_port,
                    self.hos_server_passwd.clone(),
                    self.reqwest_client.clone(),
                )
                .await
                .connections;
                let mut sessions_with_pairing_code = vec![];
                for connection in connections {
                    if connection.1 == pairing_code {
                        sessions_with_pairing_code.push(connection.0);
                    }
                }
                if sessions_with_pairing_code.is_empty() {
                    ctx.say(
                        "You have a HOS pairing code, but no client running with it. \
                    Connect your HOS client.",
                    )
                    .await
                    .unwrap();
                    return None;
                }
                if sessions_with_pairing_code.len() > 1 {
                    ctx.say("You have a HOS pairing code, but multiple clients are using it! \
                            Disconnect every client and reconnect only one, or, alternatively, do `!reset` and try again with one client and a new pairing code.").await.unwrap();
                    return None;
                }
                let session_id = sessions_with_pairing_code.first().unwrap().to_string();
                let creds = get_maloja_creds_for_sid(
                    session_id,
                    self.hos_server_ip.clone(),
                    self.hos_server_port,
                    self.hos_server_passwd.clone(),
                    self.hos_server_https,
                );
                Some(creds)
            }
            None => None,
        }
    }

    pub async fn handle_website_user(
        &self,
        formatted_user: String,
        _ctx: Context<'_>,
    ) -> Result<MalojaCredentials, Option<ParseError>> {
        let mut assigned_website: Option<String> = None;

        for result in get_websites(&self.pool, formatted_user).await {
            assigned_website = result.website;
        }
        match assigned_website {
            Some(website) => {
                let parsed = Url::parse(&website);
                match parsed {
                    Ok(parsed) => {
                        let https = parsed.scheme() == "https";
                        let ip = parsed.host_str();
                        if ip.is_none() {
                            return Err(None);
                        }
                        let creds = MalojaCredentials {
                            https,
                            skip_cert_verification: false,
                            ip: parsed.host_str().unwrap().to_string(),
                            port: parsed.port().unwrap_or(match https {
                                true => 443,
                                false => 80,
                            }),
                            path: Some(parsed.path().to_string()),
                            headers: None,
                            api_key: None,
                        };
                        Ok(creds)
                    }
                    Err(error) => Err(Some(error)),
                }
            }
            None => Err(None),
        }
    }

    pub async fn handle_lfm_user(
        &self,
        formatted_user: String,
        _ctx: Context<'_>,
    ) -> Option<LastFMUser> {
        let mut assigned_username: Option<String> = None;

        for result in get_lastfm_username(&self.pool, formatted_user).await {
            assigned_username = result.lastfm_username;
        }

        if let Some(username) = assigned_username {
            return Some(LastFMUser { username });
        }

        None
    }

    pub async fn handle_creds(
        &self,
        formatted_user: String,
        ctx: Context<'_>,
    ) -> Option<MljboardUser> {
        // prioritize website, then HOS, then Last.FM. website probably responds fastest so it comes first
        let creds = self.handle_website_user(formatted_user.clone(), ctx).await;
        if let Ok(creds) = creds {
            Some(MljboardUser::MalojaUser(creds))
        } else {
            match self
                .handle_hos_user(formatted_user.clone(), ctx)
                .await
                .map(MljboardUser::MalojaUser)
            {
                Some(hos_user) => Some(hos_user),
                None => self
                    .handle_lfm_user(formatted_user, ctx)
                    .await
                    .map(MljboardUser::LastFMUser),
            }
        }
    }
}

pub fn get_arg(content: String) -> String {
    let mut args = content.split(' ').collect::<Vec<&str>>();
    args.remove(0);
    args.join(" ")
}

#[poise::command(slash_command)]
pub async fn hos_setup(ctx: poise::Context<'_, BotData, Error>) -> Result<(), Error> {
    let formatted_user = format_user(ctx.author().clone());
    super::setups::hos_setup(ctx, &ctx.data().pool, formatted_user).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn website_setup(
    ctx: poise::Context<'_, BotData, Error>,
    #[description = "Website URL"] website: String,
) -> Result<(), Error> {
    let formatted_user = format_user(ctx.author().clone());
    super::setups::website_setup(ctx, &ctx.data().pool, formatted_user, website).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn lfm_setup(
    ctx: poise::Context<'_, BotData, Error>,
    #[description = "Last.FM username"] username: String,
) -> Result<(), Error> {
    let formatted_user = format_user(ctx.author().clone());
    super::setups::lfm_setup(ctx, &ctx.data().pool, formatted_user, username).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn reset(ctx: poise::Context<'_, BotData, Error>) -> Result<(), Error> {
    let formatted_user = format_user(ctx.author().clone());
    super::setups::reset(ctx, &ctx.data().pool, formatted_user).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn scrobbles(ctx: poise::Context<'_, BotData, Error>) -> Result<(), Error> {
    let formatted_user = format_user(ctx.author().clone());
    let user = ctx.data().handle_creds(formatted_user, ctx).await;

    if user.is_none() {
        ctx.say("You don't have a HOS pairing code or a website set up.")
            .await
            .unwrap();
        return Ok(());
    }

    super::ops::scrobbles_cmd(ctx.data().reqwest_client.clone(), user, None, ctx).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn artistscrobbles(
    ctx: poise::Context<'_, BotData, Error>,
    #[description = "Artist"] artist: String,
) -> Result<(), Error> {
    let formatted_user = format_user(ctx.author().clone());
    let user = ctx.data().handle_creds(formatted_user, ctx).await;

    if user.is_none() {
        ctx.say("You don't have a HOS pairing code or a website set up.")
            .await
            .unwrap();
        return Ok(());
    }

    super::ops::artistscrobbles_cmd(ctx.data().reqwest_client.clone(), user, None, ctx, artist)
        .await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn lfmuser(
    ctx: poise::Context<'_, BotData, Error>,
    #[description = "User"] user: String,
) -> Result<(), Error> {
    super::lastfm::lfmuser_cmd(ctx, ctx.data().lastfm_api.clone(), user).await;
    Ok(())
}
