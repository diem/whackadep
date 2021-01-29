use anyhow::Result;
use std::path::Path;
use std::sync::mpsc::Receiver;
use tracing::{error, info};

mod analysis;
pub mod db;
mod dependabot;
mod external;
mod git;
pub mod rust;

use analysis::analyze;

pub enum MetricsRequest {
    // request to refresh list of transitive dependencies
    RustDependencies { repo_url: String },
}

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
