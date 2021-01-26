use anyhow::{anyhow, Result};
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

pub async fn start(receiver: Receiver<MetricsRequest>) -> Result<()> {
    let repo_path = Path::new("diem_repo");

    println!("initializing stuff in metrics service");
    rust::cargotree::CargoTree::init_cargo_tree().await?;
    rust::cargoaudit::CargoAudit::init_cargo_audit().await?;

    println!("metrics service starting");
    for request in receiver {
        match request {
            MetricsRequest::RustDependencies { repo_url } => {
                println!("commencing rust analysis");
                match Analysis::analyze(&repo_url, &repo_path).await {
                    Ok(()) => println!("all good"),
                    Err(e) => {
                        println!("metrics failed to terminate: {}", e);
                        continue;
                    }
                };
            }
            _ => return Err(anyhow!("invalid request")),
        };
    }
    Ok(())
}
