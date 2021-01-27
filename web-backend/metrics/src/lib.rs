use anyhow::Result;
use std::path::Path;
use std::sync::mpsc::Receiver;

mod analysis;
pub mod db;
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

    println!("initializing stuff in metrics service");
    rust::cargotree::CargoTree::init_cargo_tree().await?;
    rust::cargoaudit::CargoAudit::init_cargo_audit().await?;

    println!("metrics service starting");
    for request in receiver {
        match request {
            MetricsRequest::RustDependencies { repo_url } => {
                match analyze(&repo_url, &repo_path).await {
                    Ok(()) => println!("analyze finished successfuly"),
                    Err(e) => {
                        eprintln!("metrics failed to terminate: {}", e);
                        continue;
                    }
                };
            }
        };
    }
    Ok(())
}
