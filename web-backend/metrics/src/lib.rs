//! Metrics is a library that can analyze rust dependencies in a given repository.
//! It can also be used to run a Metrics service, with the function [`start()`].

use anyhow::Result;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use tracing::{error, info};

pub mod analysis;
pub mod common;
pub mod git;
pub mod model;
pub mod rust;

use analysis::MetricsApp;

/// A request that can be sent to the Metrics service (see [`start()`]).
pub enum MetricsRequest {
    /// A request to refresh the list of rust dependencies, given a git repository.
    StartAnalysis { repo_url: String },
}

/// Initializes a metrics service with a channel [`Receiver`] and wait for requests to process.
/// Requests on that channel can be of type [`MetricsRequest`].
/// It currently only supports one query at a time,
/// and will prevent any queries from being sent when busy.
/// For this reason, you should call the sender with [`std::sync::mpsc::SyncSender::try_send()`].
pub async fn start(receiver: Receiver<MetricsRequest>) -> Result<()> {
    info!("initializing cargo tree");
    rust::cargotree::CargoTree::init_cargo_tree().await?;

    info!("initializing cargo audit");
    rust::cargoaudit::CargoAudit::init_cargo_audit().await?;

    info!("initializing cargo download");
    rust::diff::init_cargo_download().await?;

    let metrics = MetricsApp::new().await?;

    info!("metrics service started!");
    let mut repo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    repo_dir.push("repos");

    for request in receiver {
        match request {
            MetricsRequest::StartAnalysis { repo_url } => {
                match metrics.refresh(&repo_url, &repo_dir).await {
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
