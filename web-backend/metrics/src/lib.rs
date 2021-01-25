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
    println!("metrics service started");
    for request in receiver {
        match request {
            MetricsRequest::RustDependencies { repo_url } => {
                println!("commencing rust analysis");
                match Analysis::analyze(&repo_url, Path::new("diem_repo")).await {
                    Ok(()) => println!("all good"),
                    Err(e) => {
                        println!("{}", e);
                        continue;
                    }
                };
            }
            _ => return Err(anyhow!("invalid request")),
        };
    }
    Ok(())
}
