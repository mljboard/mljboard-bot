pub mod json;

use crate::hos::json::HOSConnectionList;
use mljcl::credentials::*;
use reqwest::Client;
use std::collections::HashMap;

pub async fn get_hos_connections(
    hos_server_base: String,
    hos_server_ip: String,
    hos_server_port: u16,
    hos_server_passwd: Option<String>,
    client: Client,
) -> HOSConnectionList {
    let mut builder =
        client.get(hos_server_base + &hos_server_ip + ":" + &hos_server_port.to_string() + "/list");
    if let Some(passwd) = hos_server_passwd {
        builder = builder.header("HOS-PASSWD", passwd);
    }
    let response = builder.send().await;
    response.unwrap().json::<HOSConnectionList>().await.unwrap()
}

pub fn get_maloja_creds_for_sid(
    sid: String,
    hos_server_ip: String,
    hos_server_port: u16,
    hos_server_passwd: Option<String>,
    hos_server_https: bool,
) -> MalojaCredentials {
    let mut headers: HashMap<String, String> = HashMap::new();
    if let Some(passwd) = hos_server_passwd {
        headers.insert("HOS-PASSWD".to_string(), passwd);
    }
    MalojaCredentialsBuilder::new()
    .https(hos_server_https)
    .skip_cert_verification(!hos_server_https)
    .ip(hos_server_ip)
    .port(hos_server_port)
    .path("/sid/".to_owned() + &sid)
    .headers(headers)
    .build().unwrap()
}
