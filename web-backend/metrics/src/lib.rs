//! Metrics is a library that can analyze rust dependencies in a given repository.
//! It can also be used to run a Metrics service, with the function [`start()`].

use anyhow::Result;
use std::path::Path;
use std::sync::mpsc::Receiver;
use tracing::{error, info};

pub mod analysis;
pub mod common;
pub mod db;
pub mod dependabot;
pub mod git;
pub mod rust;

use analysis::analyze;

/// A request that can be sent to the Metrics service (see [`start()`]).
pub enum MetricsRequest {
    /// A request to refresh the list of rust dependencies, given a git repository.
    RustDependencies { repo_url: String },
}

/// Initializes a metrics service with a channel [`Receiver`] and wait for requests to process.
/// Requests on that channel can be of type [`MetricsRequest`].
/// It currently only supports one query at a time,
/// and will prevent any queries from being sent when busy.
/// For this reason, you should call the sender with [`std::sync::mpsc::SyncSender::try_send()`].
pub async fn start(receiver: Receiver<MetricsRequest>) -> Result<()> {
    let repo_path = Path::new("diem_repo");

    info!("initializing cargo tree");
    rust::cargotree::CargoTree::init_cargo_tree().await?;

    info!("initializing cargo audit");
    rust::cargoaudit::CargoAudit::init_cargo_audit().await?;

    info!("metrics service started!");
    for request in receiver {
        match request {
            MetricsRequest::RustDependencies { repo_url } => {
                match analyze(&repo_url, &repo_path).await {
                    Ok(()) => info!("analyze finished successfuly"),
                    Err(e) => {
                        error!("metrics failed to terminate: {}", e);
                        continue;
                    }
                };
            }
        };
    }
    Ok(())
}
