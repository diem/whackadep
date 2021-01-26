#[macro_use]
extern crate slog;

use anyhow::{bail, Result};
use slog::Logger;
use std::path::Path;
use std::sync::mpsc::Receiver;

mod analysis;
pub mod db;
mod external;
mod git;
pub mod rust;

use analysis::Analysis;

pub enum MetricsRequest {
    // request to refresh list of transitive dependencies
    RustDependencies { repo_url: String },
}

pub async fn start(logger: Logger, receiver: Receiver<MetricsRequest>) -> Result<()> {
    let repo_path = Path::new("diem_repo");

    info!(logger, "initializing stuff in metrics service");
    rust::cargotree::CargoTree::init_cargo_tree().await?;
    rust::cargoaudit::CargoAudit::init_cargo_audit().await?;

    info!(logger, "metrics service starting");
    for request in receiver {
        match request {
            MetricsRequest::RustDependencies { repo_url } => {
                match Analysis::analyze(&repo_url, &repo_path).await {
                    Ok(()) => println!("all good"),
                    Err(e) => {
                        println!("metrics failed to terminate: {}", e);
                        continue;
                    }
                };
            }
            _ => bail!("invalid request"),
        };
    }
    Ok(())
}
