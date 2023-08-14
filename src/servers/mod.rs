use std::sync::Arc;

use tokio::join;

use crate::LookupData;

pub mod main;
pub mod qos;
pub mod redirector;
pub mod telemetry;

/// Starts and waits for all the servers
pub async fn start(target: Arc<LookupData>) {
    join!(
        main::start_server(target.clone()),
        qos::start_server(),
        redirector::start_server(),
        telemetry::start_server(target)
    );
}
