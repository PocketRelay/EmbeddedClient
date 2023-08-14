#![allow(clippy::missing_safety_doc)]

use std::sync::Arc;

use constants::SERVER_IDENT;
use futures_util::Future;
use log::{debug, error};
use native_dialog::MessageDialog;
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    sync::RwLock,
    task::{JoinHandle, JoinSet},
};
use windows_sys::Win32::System::{
    Console::{AllocConsole, FreeConsole},
    LibraryLoader::FreeLibrary,
    SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
};

pub mod constants;
pub mod hooks;
pub mod interface;
pub mod pattern;
pub mod plugin;
pub mod proxy;
pub mod servers;

static SERVERS_TASK: RwLock<Option<JoinHandle<()>>> = RwLock::const_new(None);
static TASK_SET: RwLock<Option<JoinSet<()>>> = RwLock::const_new(None);

pub async fn spawn_task<F>(task: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let mut set = TASK_SET.write().await;
    if let Some(value) = &mut *set {
        value.spawn(task);
    } else {
        error!("Failed to spawn task, task set not initialized")
    }
}

pub async fn cancel_tasks() {
    let mut set = TASK_SET.write().await;
    if let Some(value) = &mut *set {
        value.abort_all();
    }
}

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
unsafe extern "system" fn DllMain(dll_module: usize, call_reason: u32, _: *mut ()) -> bool {
    match call_reason {
        DLL_PROCESS_ATTACH => {
            std::thread::spawn(|| {
                *TASK_SET.blocking_write() = Some(JoinSet::new());

                // Create tokio async runtime
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed building the Runtime");

                // Initialize the UI
                interface::init(runtime);
            });

            // Allocate a console
            AllocConsole();

            env_logger::builder()
                .filter_level(log::LevelFilter::Debug)
                .init();

            // initialize the proxy
            proxy::init();

            // Handles the DLL being attached to the game
            unsafe { hooks::hook() };

            // Load ASI plugins
            plugin::load();
        }
        DLL_PROCESS_DETACH => {
            // free the proxied library
            if let Some(handle) = proxy::PROXY_HANDLE.take() {
                FreeLibrary(handle);
            }

            // Free the console
            FreeConsole();
        }
        _ => {}
    }

    true
}

/// Shows a native info dialog with the provided title and text
///
/// `title` The title of the dialog
/// `text`  The text of the dialog
pub fn show_info(title: &str, text: &str) {
    MessageDialog::new()
        .set_title(title)
        .set_text(text)
        .set_type(native_dialog::MessageType::Info)
        .show_alert()
        .unwrap()
}

/// Shows a native error dialog with the provided title and text
///
/// `title` The title of the dialog
/// `text`  The text of the dialog
pub fn show_error(title: &str, text: &str) {
    MessageDialog::new()
        .set_title(title)
        .set_text(text)
        .set_type(native_dialog::MessageType::Error)
        .show_alert()
        .unwrap()
}

/// Details provided by the server. These are the only fields
/// that we need the rest are ignored by this client.
#[derive(Deserialize)]
struct ServerDetails {
    /// The Pocket Relay version of the server
    version: String,
    /// Server identifier checked to ensure its a proper server
    #[serde(default)]
    ident: Option<String>,
}

/// Data from completing a lookup contains the resolved address
/// from the connection to the server as well as the server
/// version obtained from the server
#[derive(Debug, Clone)]
pub struct LookupData {
    /// The scheme used to connect to the server (e.g http or https)
    scheme: String,
    /// The host address of the server
    host: String,
    /// The server version
    version: String,
    /// The server port
    port: u16,
}

/// Errors that can occur while looking up a server
#[derive(Debug, Error)]
enum LookupError {
    /// The server url was missing the host portion
    #[error("Unable to find host portion of provided Connection URL")]
    InvalidHostTarget,
    /// The server connection failed
    #[error("Failed to connect to server")]
    ConnectionFailed(reqwest::Error),
    /// The server gave an invalid response likely not a PR server
    #[error("Invalid server response")]
    InvalidResponse(reqwest::Error),
    #[error("Server identifier was incorrect (Not a PocketRelay server?)")]
    NotPocketRelay,
}

/// Attempts to connect to the provided target server, if the connection
/// succeeds then the local server will start
///
/// `target` The target to use
async fn try_start_servers(target: String) -> Result<Arc<LookupData>, LookupError> {
    let result = try_lookup_host(target).await?;
    let result = Arc::new(result);
    // Start the servers
    let handle = tokio::spawn(servers::start(result.clone()));

    let write = &mut *SERVERS_TASK.write().await;
    *write = Some(handle);

    Ok(result)
}

async fn stop_servers() {
    if let Some(handle) = SERVERS_TASK.write().await.take() {
        debug!("Stopping servers");
        handle.abort();
    }
    cancel_tasks().await;
}

/// Attempts to connect to the Pocket Relay HTTP server at the provided
/// host. Will make a connection to the /api/server endpoint and if the
/// response is a valid ServerDetails message then the server is
/// considered valid.
///
/// `host` The host to try and lookup
async fn try_lookup_host(host: String) -> Result<LookupData, LookupError> {
    let mut url = String::new();

    // Fill in missing host portion
    if !host.starts_with("http://") && !host.starts_with("https://") {
        url.push_str("http://");
        url.push_str(&host)
    } else {
        url.push_str(&host);
    }

    if !host.ends_with('/') {
        url.push('/')
    }

    url.push_str("api/server");

    let response = reqwest::get(url)
        .await
        .map_err(LookupError::ConnectionFailed)?;

    let url = response.url();
    let scheme = url.scheme().to_string();

    let port = url.port_or_known_default().unwrap_or(80);
    let host = match url.host() {
        Some(value) => value.to_string(),
        None => return Err(LookupError::InvalidHostTarget),
    };

    let details = response
        .json::<ServerDetails>()
        .await
        .map_err(LookupError::InvalidResponse)?;

    // Handle invalid server ident
    if details.ident.is_none() || details.ident.is_some_and(|value| value != SERVER_IDENT) {
        return Err(LookupError::NotPocketRelay);
    }

    Ok(LookupData {
        scheme,
        host,
        port,
        version: details.version,
    })
}
