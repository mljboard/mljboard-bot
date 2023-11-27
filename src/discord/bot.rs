use crate::hos::*;
use futures::stream::StreamExt;
use mljcl::history::numscrobbles_async;
use mljcl::range::Range;
use mljcl::MalojaCredentials;
use mongodb::bson::{doc, Document};
use mongodb::options::{DeleteOptions, InsertOneOptions};
use mongodb::{Cursor, Database};
use serenity::async_trait;
use serenity::model::channel::PrivateChannel;
use serenity::model::gateway::Ready;
use serenity::model::prelude::Message;
use serenity::model::user::User;
use serenity::prelude::*;

#[derive(Clone, Debug)]
struct Handler {
    pub db: Database,
    pub hos_server_ip: String,
    pub hos_server_port: u16,
    pub hos_server_passwd: Option<String>,
    pub reqwest_client: reqwest::Client,
    pub lastfm_api: Option<String>,
}

pub fn format_user(user: User) -> String {
    format!("{}#{}", user.name, user.discriminator)
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

pub async fn handle_hos_user(
    pairing_code_cursor: &mut Cursor<Document>,
    formatted_user: String,
    ctx: Context,
    handler: Handler,
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
            let connections = get_hos_connections(
                // TODO: allow https
                "http://".to_string(),
                handler.hos_server_ip.clone(),
                handler.hos_server_port,
                handler.hos_server_passwd.clone(),
                handler.reqwest_client.clone(),
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
                msg.reply_ping(ctx.clone(), "You have a HOS pairing code, but no client running with it. Connect your HOS client.").await.unwrap();
                return None;
            }
            if sessions_with_pairing_code.len() > 1 {
                msg.reply_ping(ctx.clone(),
                        "You have a HOS pairing code, but multiple clients are using it! Disconnect every client and reconnect only one, or, alternatively, do `!reset` and try again with one client and a new pairing code.").await.unwrap();
                return None;
            }
            let session_id = sessions_with_pairing_code.first().unwrap().to_string();
            let creds = get_maloja_creds_for_sid(
                session_id,
                handler.hos_server_ip.clone(),
                handler.hos_server_port,
                handler.hos_server_passwd.clone(),
            );
            Some(creds)
        }
        None => {
            // TODO: check for website, allow selecting a website
            msg.reply_ping(
                ctx.clone(),
                "You don't have a HOS pairing code or a website set up.",
            )
            .await
            .unwrap();
            return None;
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let formatted_user = format_user(msg.author.clone());
        let pairing_codes = self.db.collection::<Document>("discord_pairing_codes");
        let mut pairing_code_cursor = pairing_codes
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
                    dm_channel.send_message(ctx, |m| {
                            m.content(format!("You have been assigned the pairing code `{}`. Make sure to pass this to your HOS client.", key))
                        }).await.unwrap();
                } else {
                    dm_channel
                            .send_message(ctx, |m| {
                                m.content(
                                    "You've already made a pairing code, or you have a website linked. Do `!reset` to revoke the code and/or remove the website.",
                                )
                            })
                            .await
                            .unwrap();
                }
            }
        } else if msg.content == "!reset" {
            if let Some(dm_channel) = dm_channel!(msg, ctx) {
                while let Some(pair) = pairing_code_cursor.next().await {
                    if pair.clone().unwrap().get_str("user").unwrap() == formatted_user.clone() {
                        dm_channel.send_message(ctx.clone(), |m| {
                            m.content(format!("Removing your pairing code `{}` from mljboard's database. Run `!setup` to be issued a new one.", pair.clone().unwrap().get_str("pairing_code").unwrap_or("[none]")))
                        }).await.unwrap();
                        pairing_codes
                            .delete_one(pair.unwrap(), DeleteOptions::builder().build())
                            .await
                            .unwrap();
                        return;
                    }
                }
                dm_channel
                    .send_message(ctx.clone(), |m| {
                        m.content("We couldn't find any pairing codes that were yours.")
                    })
                    .await
                    .unwrap();
            }
        } else if msg.content == "!scrobbles" {
            if let Some(creds) = handle_hos_user(
                &mut pairing_code_cursor,
                formatted_user,
                ctx.clone(),
                self.clone(),
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
                    .send_message(ctx, |m| {
                        m.embed(|e| {
                            e.title(format!("{}'s scrobbles", msg.author.name))
                                .field("All time", all_time_scrobbles, false)
                                .field("This year", this_year_scrobbles, false)
                        })
                    })
                    .await
                    .unwrap();
            }
        } else if msg.content.starts_with("!artistscrobbles") {
            let mut args = msg.content.split(" ").collect::<Vec<&str>>();
            args.remove(0);
            let args = args.join(" ");

            if let Some(creds) = handle_hos_user(
                &mut pairing_code_cursor,
                formatted_user,
                ctx.clone(),
                self.clone(),
                msg.clone(),
            )
            .await
            {
                let all_time_scrobbles = numscrobbles_async(
                    Some(args.clone()),
                    Range::AllTime,
                    creds.clone(),
                    self.reqwest_client.clone(),
                )
                .await
                .unwrap();
                msg.channel_id
                    .send_message(ctx, |m| {
                        m.embed(|e| {
                            e.title(format!("{}'s scrobbles for {}", msg.author.name, args))
                                .field("All time", all_time_scrobbles, false)
                        })
                    })
                    .await
                    .unwrap();
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        log::info!(
            "{}#{} is connected!",
            ready.user.name,
            ready.user.discriminator
        );
    }
}

pub async fn run_bot(
    token: String,
    db: Database,
    hos_server_ip: String,
    hos_server_port: u16,
    hos_server_passwd: Option<String>,
    lastfm_api: Option<String>,
) {
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            db,
            hos_server_ip,
            hos_server_port,
            hos_server_passwd,
            reqwest_client: reqwest::Client::builder().build().unwrap(),
            lastfm_api,
        })
        .await
        .expect("Error creating Discord client");

    if let Err(why) = client.start().await {
        log::error!("Discord client error: {why:?}");
    }
}
