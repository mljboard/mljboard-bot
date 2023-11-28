use clap::{Arg, Command};
use mongodb::{options::CreateCollectionOptions, Database};
use std::thread::park;

pub async fn run(
    bot_token: String,
    hos_server_ip: String,
    hos_server_port: u16,
    hos_server_passwd: Option<String>,
    hos_server_https: bool,
    db: Database,
    lastfm_api: Option<String>,
) -> serenity::client::ClientBuilder {
    if !(db
        .list_collection_names(None)
        .await
        .unwrap()
        .contains(&"discord_pairing_codes".to_string()))
    {
        log::warn!("Pairing code collection not found in DB. Creating.");
        db.create_collection(
            "discord_pairing_codes",
            CreateCollectionOptions::builder().build(),
        )
        .await
        .expect("Failed to create Discord pairing codes collection");
    }

    if !(db
        .list_collection_names(None)
        .await
        .unwrap()
        .contains(&"discord_websites".to_string()))
    {
        log::warn!("Website collection not found in DB. Creating.");
        db.create_collection(
            "discord_websites",
            CreateCollectionOptions::builder().build(),
        )
        .await
        .expect("Failed to create Discord websites collection");
    }

    log::info!("Finished checking for collections. Spawning bot thread.");

    return mljboard_bot::discord::bot::build_bot(
        bot_token,
        db,
        hos_server_ip,
        hos_server_port,
        hos_server_passwd,
        hos_server_https,
        lastfm_api,
    )
    .await;
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
            Arg::new("mongo_location")
                .short('m')
                .value_name("MONGO")
                .help("MongoDB server location. Format: mongodb://user:password@server/path"),
        )
        .arg(
            Arg::new("mongo_db")
                .short('p')
                .value_name("MONGO_DB")
                .help("MongoDB database name"),
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

    let mongo_db_creds: String = matches
        .get_one::<String>("mongo_location")
        .expect("MongoDB creds required")
        .to_string();

    let mongo_db: String = matches
        .get_one::<String>("mongo_db")
        .expect("MongoDB DB name required")
        .to_string();

    let lastfm_api: Option<String> = matches.get_one::<String>("lfm_api").map(|x| x.to_string());

    log::info!("Connecting to database");

    let db = mljboard_bot::db::mongo::start_db(mongo_db_creds, mongo_db).await;

    log::info!("Connected to database");

    let client_builder = run(
        bot_token,
        hos_server_ip,
        hos_server_port,
        hos_server_passwd,
        hos_server_https,
        db,
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
    #[shuttle_shared_db::MongoDb] db: Database,
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
        db,
        lastfm_api,
    )
    .await;
    let client: serenity::Client = client_builder.await.unwrap();

    Ok(shuttle_serenity::SerenityService(client))
}
