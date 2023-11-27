pub mod json;

use crate::hos::json::HOSConnectionList;
use mljcl::MalojaCredentials;
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
    match hos_server_passwd {
        Some(passwd) => {
            builder = builder.header("HOS-PASSWD", passwd);
        }
        None => (),
    }
    let response = builder.send().await;
    response.unwrap().json::<HOSConnectionList>().await.unwrap()
}

pub fn get_maloja_creds_for_sid(
    sid: String,
    hos_server_ip: String,
    hos_server_port: u16,
    hos_server_passwd: Option<String>,
) -> MalojaCredentials {
    let mut headers: HashMap<String, String> = HashMap::new();
    if let Some(passwd) = hos_server_passwd {
        headers.insert("HOS-PASSWD".to_string(), passwd);
    }
    MalojaCredentials {
        https: false,
        skip_cert_verification: true,
        ip: hos_server_ip,
        port: hos_server_port,
        path: Some("/sid/".to_owned() + &sid),
        headers: Some(headers),
        api_key: None,
    }
}
