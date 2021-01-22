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
    Dependencies,
}

const REPO_URL: &str = "https://github.com/diem/diem.git";

pub async fn start(receiver: Receiver<MetricsRequest>) -> Result<()> {
    for request in receiver {
        match request {
            MetricsRequest::Dependencies => {
                println!("commencing analysis");

                let metrics = match Analysis::analyze(REPO_URL, Path::new("diem_repo")).await {
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
