use crate::{constants::MAIN_PORT, show_error, spawn_task, LookupData};
use log::{debug, error};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client,
};
use std::{net::Ipv4Addr, process::exit, sync::Arc};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
};

/// Starts the main server proxy. This creates a connection to the Pocket Relay
/// which is upgraded and then used as the main connection fro the game.
pub async fn start_server(target: Arc<LookupData>) {
    // Initializing the underlying TCP listener
    let listener = match TcpListener::bind((Ipv4Addr::UNSPECIFIED, MAIN_PORT)).await {
        Ok(value) => value,
        Err(err) => {
            let text = format!("Failed to start main: {}", err);
            show_error("Failed to start", &text);
            exit(1);
        }
    };

    // Accept incoming connections
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(value) => value,
            Err(err) => {
                error!("Failed to accept main connection: {}", err);
                break;
            }
        };

        debug!("Main connection ->");

        // Spawn off a new handler for the connection
        spawn_task(handle_blaze(stream, target.clone())).await;
    }
}

/// Header for the Pocket Relay connection scheme used by the client
const HEADER_SCHEME: &str = "X-Pocket-Relay-Scheme";
/// Header for the Pocket Relay connection port used by the client
const HEADER_PORT: &str = "X-Pocket-Relay-Port";
/// Header for the Pocket Relay connection host used by the client
const HEADER_HOST: &str = "X-Pocket-Relay-Host";
/// Endpoint for upgrading the server connection
const UPGRADE_ENDPOINT: &str = "/api/server/upgrade";

async fn handle_blaze(mut client: TcpStream, target: Arc<LookupData>) {
    // Create the upgrade URL
    let mut url = String::new();
    url.push_str(&target.scheme);
    url.push_str("://");
    url.push_str(&target.host);
    url.push_str(UPGRADE_ENDPOINT);

    // Create the HTTP Upgrade headers
    let mut headers = HeaderMap::new();
    headers.insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
    headers.insert(header::UPGRADE, HeaderValue::from_static("blaze"));

    // Append the schema header
    if let Ok(scheme_value) = HeaderValue::from_str(&target.scheme) {
        headers.insert(HEADER_SCHEME, scheme_value);
    }

    // Append the port header
    headers.insert(HEADER_PORT, HeaderValue::from(target.port));

    // Append the host header
    if let Ok(host_value) = HeaderValue::from_str(&target.host) {
        headers.insert(HEADER_HOST, host_value);
    }

    debug!("Connecting pipe to Pocket Relay server");

    // Create the request
    let request = Client::new().get(url).headers(headers).send();

    // Await the server response to the request
    let response = match request.await {
        Ok(value) => value,
        Err(err) => {
            error!("Failed to get server pipe response: {}", err);
            return;
        }
    };

    // Server connection gained through upgrading the client
    let mut server = match response.upgrade().await {
        Ok(value) => value,
        Err(err) => {
            error!("Failed to upgrade connection pipe: {}", err);
            return;
        }
    };

    // Copy the data between the connection
    let _ = copy_bidirectional(&mut client, &mut server).await;
}
