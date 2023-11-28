use crate::hos::*;
use core::num::NonZeroU16;
use futures::stream::StreamExt;
use mljcl::history::numscrobbles_async;
use mljcl::range::Range;
use mljcl::MalojaCredentials;
use mongodb::bson::{doc, Document};
use mongodb::options::{DeleteOptions, InsertOneOptions};
use mongodb::{Cursor, Database};
use serenity::all::CreateMessage;
use serenity::async_trait;
use serenity::builder::CreateEmbed;
use serenity::model::channel::PrivateChannel;
use serenity::model::gateway::Ready;
use serenity::model::prelude::Message;
use serenity::model::user::User;
use serenity::prelude::*;
use url::{ParseError, Url};

#[derive(Clone, Debug)]
struct Handler {
    pub db: Database,
    pub hos_server_ip: String,
    pub hos_server_port: u16,
    pub hos_server_passwd: Option<String>,
    pub hos_server_https: bool,
    pub reqwest_client: reqwest::Client,
    pub lastfm_api: Option<String>,
}

fn option_nonzerou16_to_u16(input: Option<NonZeroU16>) -> u16 {
    // we want to keep the `#0` after the user since any databases created
    // before this commit will have it
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

macro_rules! dm_channel {
    ( $msg:ident, $ctx:ident ) => {
        try_dm_channel($msg.clone().author, Some($msg.clone()), $ctx.clone()).await
    };
}

impl Handler {
    pub async fn handle_hos_user(
        &self,
        pairing_code_cursor: &mut Cursor<Document>,
        formatted_user: String,
        ctx: Context,
        msg: Message,
    ) -> Option<MalojaCredentials> {
        let mut assigned_pairing_code: Option<String> = None;
        while let Some(pair) = pairing_code_cursor.next().await {
            if pair.clone().unwrap().get_str("user").unwrap() == formatted_user.clone() {
                assigned_pairing_code = Some(
                    pair.clone()
                        .unwrap()
                        .get_str("pairing_code")
                        .unwrap()
                        .to_string(),
                );
            }
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
        website_cursor: &mut Cursor<Document>,
        formatted_user: String,
        _ctx: Context,
        _msg: Message,
    ) -> Result<MalojaCredentials, Option<ParseError>> {
        let mut assigned_website: Option<String> = None;
        while let Some(pair) = website_cursor.next().await {
            if pair.clone().unwrap().get_str("user").unwrap() == formatted_user.clone() {
                assigned_website = Some(
                    pair.clone()
                        .unwrap()
                        .get_str("website")
                        .unwrap()
                        .to_string(),
                );
            }
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
        pairing_code_cursor: &mut Cursor<Document>,
        website_cursor: &mut Cursor<Document>,
        formatted_user: String,
        ctx: Context,
        msg: Message,
    ) -> Option<MalojaCredentials> {
        let creds = self
            .handle_website_user(
                website_cursor,
                formatted_user.clone(),
                ctx.clone(),
                msg.clone(),
            )
            .await;
        if creds.is_err() {
            self.handle_hos_user(
                pairing_code_cursor,
                formatted_user,
                ctx.clone(),
                msg.clone(),
            )
            .await
        } else {
            Some(creds.unwrap())
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
        let pairing_codes = self.db.collection::<Document>("discord_pairing_codes");
        let websites = self.db.collection::<Document>("discord_websites");
        let mut pairing_code_cursor = pairing_codes
            .find(doc! {}, mongodb::options::FindOptions::builder().build())
            .await
            .unwrap();
        let mut website_cursor = websites
            .find(doc! {}, mongodb::options::FindOptions::builder().build())
            .await
            .unwrap();

        if msg.content == "!hos_setup" {
            if let Some(dm_channel) = dm_channel!(msg, ctx) {
                let mut match_found = false;
                while let Some(pair) = pairing_code_cursor.next().await {
                    if pair.unwrap().get_str("user").unwrap() == formatted_user.clone() {
                        match_found = true;
                    }
                }
                if !match_found {
                    let key = crate::generate_api_key();
                    //TODO: check unique
                    pairing_codes
                        .insert_one(
                            doc! {"user": formatted_user.clone(), "pairing_code": key.clone()},
                            InsertOneOptions::builder().build(),
                        )
                        .await
                        .unwrap();
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
            }
        } else if msg.content.starts_with("!website_setup") {
            let arg = get_arg(msg.clone().content);
            let mut match_found = false;
            while let Some(pair) = website_cursor.next().await {
                if pair.unwrap().get_str("user").unwrap() == formatted_user.clone() {
                    match_found = true;
                }
            }
            if match_found {
                msg.reply_ping(
                    ctx.clone(),
                    "You've already set a website. Do `!reset` to remove it.",
                )
                .await
                .unwrap();
            }
            if !arg.is_empty() {
                if !(arg.starts_with("http://") || arg.starts_with("https://")) {
                    msg.reply_ping(ctx.clone(), "Remember that your website has to start with `http://` or `https://`. Try again with \
                    one of those two, and keep in mind if you're using https you cannot use an invalid certificate.").await.unwrap();
                    return;
                }
                msg.reply_ping(ctx.clone(), format!("Setting your website to {}.", arg))
                    .await
                    .unwrap();
                websites
                    .insert_one(
                        doc! {"user": formatted_user.clone(), "website": arg.clone()},
                        InsertOneOptions::builder().build(),
                    )
                    .await
                    .unwrap();
            } else {
                msg.reply_ping(ctx, "No website provided.").await.unwrap();
            }
        } else if msg.content == "!reset" {
            if let Some(dm_channel) = dm_channel!(msg, ctx) {
                while let Some(pair) = website_cursor.next().await {
                    if pair.clone().unwrap().get_str("user").unwrap() == formatted_user.clone() {
                        dm_channel
                            .send_message(
                                ctx.clone(),
                                CreateMessage::new().content(format!(
                                    "Removing your website `{}` from mljboard's database. \
                            Run `!site_setup` to assign yourself one.",
                                    pair.clone().unwrap().get_str("website").unwrap_or("[none]")
                                )),
                            )
                            .await
                            .unwrap();
                        websites
                            .delete_one(pair.unwrap(), DeleteOptions::builder().build())
                            .await
                            .unwrap();
                    }
                }
                while let Some(pair) = pairing_code_cursor.next().await {
                    if pair.clone().unwrap().get_str("user").unwrap() == formatted_user.clone() {
                        dm_channel
                            .send_message(
                                ctx.clone(),
                                CreateMessage::new().content(format!(
                                    "Removing your pairing code `{}` from mljboard's database. \
                            Run `!hos_setup` to be issued a new one.",
                                    pair.clone()
                                        .unwrap()
                                        .get_str("pairing_code")
                                        .unwrap_or("[none]")
                                )),
                            )
                            .await
                            .unwrap();
                        pairing_codes
                            .delete_one(pair.unwrap(), DeleteOptions::builder().build())
                            .await
                            .unwrap();
                        return;
                    }
                }
                dm_channel
                    .send_message(
                        ctx.clone(),
                        CreateMessage::new()
                            .content("We couldn't find any pairing codes that were yours."),
                    )
                    .await
                    .unwrap();
            }
        } else if msg.content == "!scrobbles" {
            if let Some(creds) = self
                .handle_creds(
                    &mut pairing_code_cursor,
                    &mut website_cursor,
                    formatted_user,
                    ctx.clone(),
                    msg.clone(),
                )
                .await
            {
                let all_time_scrobbles = numscrobbles_async(
                    None,
                    Range::AllTime,
                    creds.clone(),
                    self.reqwest_client.clone(),
                )
                .await
                .unwrap();
                let this_year_scrobbles = numscrobbles_async(
                    None,
                    Range::In("thisyear".to_string()),
                    creds,
                    self.reqwest_client.clone(),
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
        } else if msg.content.starts_with("!artistscrobbles") {
            let arg = get_arg(msg.clone().content);

            if let Some(creds) = self
                .handle_creds(
                    &mut pairing_code_cursor,
                    &mut website_cursor,
                    formatted_user,
                    ctx.clone(),
                    msg.clone(),
                )
                .await
            {
                let all_time_scrobbles = numscrobbles_async(
                    Some(arg.clone()),
                    Range::AllTime,
                    creds.clone(),
                    self.reqwest_client.clone(),
                )
                .await
                .unwrap();
                msg.channel_id
                    .send_message(
                        ctx,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title(format!("{}'s scrobbles", msg.author.name))
                                .field("All time", all_time_scrobbles.to_string(), false),
                        ),
                    )
                    .await
                    .unwrap();
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        log::info!("{} is connected!", format_user(ready.user.into()));
    }
}

pub async fn build_bot(
    token: String,
    db: Database,
    hos_server_ip: String,
    hos_server_port: u16,
    hos_server_passwd: Option<String>,
    hos_server_https: bool,
    lastfm_api: Option<String>,
) -> serenity::client::ClientBuilder {
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(&token, intents).event_handler(Handler {
        db,
        hos_server_ip,
        hos_server_port,
        hos_server_passwd,
        hos_server_https,
        reqwest_client: reqwest::Client::builder().build().unwrap(),
        lastfm_api,
    })
}
