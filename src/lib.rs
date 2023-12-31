pub mod db;
pub mod discord;
pub mod hos;
pub mod lfm;

pub fn generate_api_key() -> String {
    use prefixed_api_key::PrefixedApiKeyController;

    let mut key_controller = PrefixedApiKeyController::configure()
        .prefix("mljboard".to_owned())
        .seam_defaults()
        .short_token_length(8)
        .long_token_length(24)
        .finalize()
        .unwrap();

    key_controller.generate_key().to_string()
}
