#[allow(unused_imports)]
use clap::{Arg, Command};
use mljboard_bot::discord::bot::*;
use poise::serenity_prelude::*;
use sqlx::{Executor, PgPool};
#[allow(unused_imports)]
use std::env;

pub async fn run(
    bot_token: String,
    hos_server_ip: String,
    hos_server_port: u16,
    hos_server_passwd: Option<String>,
    hos_server_https: bool,
    pool: PgPool,
    lastfm_api: Option<String>,
) -> poise::serenity_prelude::client::ClientBuilder {
    log::info!("Connecting to database");

    pool.execute(include_str!("../schema.sql")).await.unwrap();

    log::info!("Finished checking for tables. Spawning bot thread.");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                hos_setup(),
                website_setup(),
                lfm_setup(),
                reset(),
                scrobbles(),
                artistscrobbles(),
                lfmuser(),
                grid(),
            ],
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                log::info!("{} is connected!", format_user(ready.user.clone().into()));
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(BotData {
                    pool,
                    hos_server_ip,
                    hos_server_port,
                    hos_server_passwd,
                    hos_server_https,
                    reqwest_client: reqwest::Client::builder().build().unwrap(),
                    lastfm_api,
                })
            })
        })
        .build();

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    serenity::client::ClientBuilder::new(bot_token, intents).framework(framework)
}

#[cfg(not(feature = "shuttle"))]
#[tokio::main]
pub async fn main() {
    env_logger::init_from_env(
        env_logger::Env::new().default_filter_or("info,tracing=off,serenity=off"),
    );

    let matches = Command::new("mljboard-bot")
        .arg(
            Arg::new("bot_token")
                .short('d')
                .value_name("BOT_TOKEN")
                .help("Discord bot token"),
        )
        .arg(
            Arg::new("hos_ip")
                .short('j')
                .value_name("HOS_IP")
                .help("HOS server IP address"),
        )
        .arg(
            Arg::new("hos_port")
                .short('k')
                .value_name("HOS_PORT")
                .help("HOS server port"),
        )
        .arg(
            Arg::new("hos_passwd")
                .short('s')
                .value_name("HOS_PASSWD")
                .help("HOS server password"),
        )
        .arg(
            Arg::new("hos-https")
                .long("hos-https")
                .value_name("HOS-HTTPS")
                .num_args(0)
                .help("Enable if your HOS server uses HTTPS. OFF by default."),
        )
        .arg(
            Arg::new("lfm_api")
                .short('l')
                .value_name("LFM_API")
                .help("Last.FM API key, for operations between mljboard-bot and Last.FM"),
        )
        .get_matches();

    let bot_token = matches.get_one::<String>("bot_token").unwrap().to_string();

    let hos_server_ip: String = matches
        .get_one::<String>("hos_ip")
        .expect("HOS IP required")
        .to_string();

    let hos_server_port: u16 = matches
        .get_one::<String>("hos_port")
        .expect("HOS port required")
        .parse::<u16>()
        .expect("Invalid HOS port");

    let hos_server_passwd: Option<String> = matches
        .get_one::<String>("hos_passwd")
        .map(|x| x.to_string());

    let hos_server_https: bool = matches.get_flag("hos-https");

    let postgres_url: String = env::var("DATABASE_URL").expect("Need a postgres database url");

    let lastfm_api: Option<String> = matches.get_one::<String>("lfm_api").map(|x| x.to_string());

    let pool = mljboard_bot::db::postgres::start_db(postgres_url)
        .await
        .expect("Postgres connection failed");

    let client_builder = run(
        bot_token,
        hos_server_ip,
        hos_server_port,
        hos_server_passwd,
        hos_server_https,
        pool,
        lastfm_api,
    )
    .await;

    let mut client = client_builder.await.unwrap();

    client.start().await.unwrap();
}

#[cfg(feature = "shuttle")]
use shuttle_secrets::SecretStore;

#[cfg(feature = "shuttle")]
#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::Postgres] pool: PgPool,
) -> shuttle_serenity::ShuttleSerenity {
    let bot_token: String = secret_store.get("BOT_TOKEN").unwrap();
    let hos_server_ip: String = secret_store.get("HOS_IP").unwrap();
    let hos_server_port: u16 = secret_store
        .get("HOS_PORT")
        .unwrap()
        .parse::<u16>()
        .unwrap();
    let hos_server_passwd: Option<String> = secret_store.get("HOS_PASSWD");
    let hos_server_https: bool = secret_store.get("HOS_HTTPS").unwrap() == "yes"; // hoping for a `bool` option in the secret store
    let lastfm_api = secret_store.get("LFM_API");

    let client_builder = run(
        bot_token,
        hos_server_ip,
        hos_server_port,
        hos_server_passwd,
        hos_server_https,
        pool,
        lastfm_api,
    )
    .await;
    let client: serenity::Client = client_builder.await.unwrap();

    Ok(shuttle_serenity::SerenityService(client))
}
