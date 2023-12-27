use crate::db::postgres::{get_discord_pairing_code, get_websites};
use crate::hos::*;
use core::num::NonZeroU16;
use mljcl::MalojaCredentials;
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::model::prelude::Message;
use serenity::model::user::User;
use serenity::prelude::*;
use sqlx::PgPool;
use url::{ParseError, Url};

#[derive(Clone, Debug)]
struct Handler {
    pub pool: PgPool,
    pub hos_server_ip: String,
    pub hos_server_port: u16,
    pub hos_server_passwd: Option<String>,
    pub hos_server_https: bool,
    pub reqwest_client: reqwest::Client,
    pub lastfm_api: Option<String>,
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

impl Handler {
    pub async fn handle_hos_user(
        &self,
        formatted_user: String,
        ctx: Context,
        msg: Message,
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
                    msg.reply_ping(
                        ctx.clone(),
                        "You have a HOS pairing code, but no client running with it. \
                    Connect your HOS client.",
                    )
                    .await
                    .unwrap();
                    return None;
                }
                if sessions_with_pairing_code.len() > 1 {
                    msg.reply_ping(ctx.clone(),
                            "You have a HOS pairing code, but multiple clients are using it! \
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
            None => {
                msg.reply_ping(
                    ctx.clone(),
                    "You don't have a HOS pairing code or a website set up.",
                )
                .await
                .unwrap();
                None
            }
        }
    }

    pub async fn handle_website_user(
        &self,
        formatted_user: String,
        _ctx: Context,
        _msg: Message,
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

    pub async fn handle_creds(
        &self,
        formatted_user: String,
        ctx: Context,
        msg: Message,
    ) -> Option<MalojaCredentials> {
        let creds = self
            .handle_website_user(formatted_user.clone(), ctx.clone(), msg.clone())
            .await;
        if let Ok(creds) = creds {
            Some(creds)
        } else {
            self.handle_hos_user(formatted_user, ctx.clone(), msg.clone())
                .await
        }
    }
}

pub fn get_arg(content: String) -> String {
    let mut args = content.split(' ').collect::<Vec<&str>>();
    args.remove(0);
    args.join(" ")
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let formatted_user = format_user(msg.author.clone());

        if msg.content == "!hos_setup" {
            super::setups::hos_setup(ctx.clone(), msg.clone(), &self.pool, formatted_user).await;
        } else if msg.content.starts_with("!website_setup") {
            let arg = get_arg(msg.clone().content);
            super::setups::website_setup(ctx.clone(), msg.clone(), &self.pool, formatted_user, arg)
                .await;
        } else if msg.content == "!reset" {
            super::setups::reset(ctx.clone(), msg.clone(), &self.pool, formatted_user).await;
        } else if msg.content == "!scrobbles" {
            let creds = self
                .handle_creds(formatted_user, ctx.clone(), msg.clone())
                .await;

            super::ops::scrobbles_cmd(msg.clone(), self.reqwest_client.clone(), creds, ctx.clone())
                .await;
        } else if msg.content.starts_with("!artistscrobbles") {
            let arg = get_arg(msg.clone().content);
            let creds = self
                .handle_creds(formatted_user, ctx.clone(), msg.clone())
                .await;

            super::ops::artistscrobbles_cmd(
                msg.clone(),
                self.reqwest_client.clone(),
                creds,
                ctx.clone(),
                arg,
            )
            .await;
        } else if msg.content.starts_with("!lfmuser") {
            let arg = get_arg(msg.clone().content);
            super::lastfm::lfmuser_cmd(ctx.clone(), msg, self.lastfm_api.clone(), arg).await;
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        log::info!("{} is connected!", format_user(ready.user.into()));
    }
}

pub async fn build_bot(
    token: String,
    pool: PgPool,
    hos_server_ip: String,
    hos_server_port: u16,
    hos_server_passwd: Option<String>,
    hos_server_https: bool,
    lastfm_api: Option<String>,
) -> serenity::client::ClientBuilder {
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(token, intents).event_handler(Handler {
        pool,
        hos_server_ip,
        hos_server_port,
        hos_server_passwd,
        hos_server_https,
        reqwest_client: reqwest::Client::builder().build().unwrap(),
        lastfm_api,
    })
}
