use sqlx::PgPool;

pub async fn start_db(postgres_url: String) -> Result<PgPool, sqlx::Error> {
    PgPool::connect(&postgres_url).await
}

pub struct DiscordWebsiteUser {
    pub discord_username: Option<String>,
    pub website: Option<String>,
}

pub struct DiscordPairingCodeUser {
    pub discord_username: Option<String>,
    pub pairing_code: Option<String>,
}

pub async fn get_websites(pool: &PgPool, formatted_user: String) -> Vec<DiscordWebsiteUser> {
    sqlx::query_as!(
        DiscordWebsiteUser,
        r#"
        SELECT * FROM discord_websites
        WHERE discord_username = $1
        "#,
        formatted_user
    )
    .fetch_all(pool)
    .await
    .expect("Failed to query DB for websites")
    .into_iter()
    .collect()
}

pub async fn get_discord_pairing_code(
    pool: &PgPool,
    formatted_user: String,
) -> Vec<DiscordPairingCodeUser> {
    sqlx::query_as!(
        DiscordPairingCodeUser,
        r#"
        SELECT * FROM discord_pairing_codes
        WHERE discord_username = $1
        "#,
        formatted_user
    )
    .fetch_all(pool)
    .await
    .expect("Failed to query DB for pairing codes")
}

pub async fn insert_website(pool: &PgPool, formatted_user: String, website: String) {
    let _ = sqlx::query!(
        r#"
            INSERT INTO discord_websites
            VALUES ( $1, $2 )
            "#,
        formatted_user,
        website
    )
    .execute(pool)
    .await
    .expect("Failed to add website to DB");
}

pub async fn insert_discord_pairing_code(pool: &PgPool, formatted_user: String, key: String) {
    let _ = sqlx::query!(
        r#"
        INSERT INTO discord_pairing_codes
        VALUES ( $1, $2 )
        "#,
        formatted_user,
        key
    )
    .execute(pool)
    .await
    .expect("Failed to add pairing code to DB");
}

pub async fn delete_discord_pairing_code(pool: &PgPool, formatted_user: String) -> u64 {
    sqlx::query!(
        r#"
        DELETE FROM discord_pairing_codes
        WHERE discord_username = $1
        "#,
        formatted_user
    )
    .execute(pool)
    .await
    .expect("Failed to delete pairing code")
    .rows_affected()
}

pub async fn delete_website(pool: &PgPool, formatted_user: String) -> u64 {
    sqlx::query!(
        r#"
        DELETE FROM discord_websites
        WHERE discord_username = $1
        "#,
        formatted_user
    )
    .execute(pool)
    .await
    .expect("Failed to delete website")
    .rows_affected()
}
