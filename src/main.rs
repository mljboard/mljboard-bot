use clap::{Arg, Command};
use mongodb::options::CreateCollectionOptions;
use std::thread::park;

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

    tokio::task::spawn(mljboard_bot::discord::bot::run_bot(
        matches.get_one::<String>("bot_token").unwrap().to_string(),
        db,
        hos_server_ip,
        hos_server_port,
        hos_server_passwd,
        lastfm_api,
    ));
    park();
}
