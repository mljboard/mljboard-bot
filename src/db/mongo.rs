use mongodb::options::ClientOptions;
use mongodb::Client;
use mongodb::Database;

pub async fn start_db(mongo_db_creds: String, mongo_db: String) -> Database {
    let client_options: ClientOptions = ClientOptions::parse(mongo_db_creds).await.unwrap();

    let client = Client::with_options(client_options).unwrap();

    client.database(&mongo_db)
}
