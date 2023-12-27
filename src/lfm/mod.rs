use lastfm::Client;

pub async fn get_lastfm_client(username: String, lastfm_api: String) -> Client<String, String> {
    Client::builder()
        .api_key(lastfm_api)
        .username(username)
        .build()
}
