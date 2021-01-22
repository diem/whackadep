use anyhow::{anyhow, Result};
use metrics::Metrics;
use std::path::Path;
use std::sync::mpsc::Receiver;

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

                let metrics = match Metrics::new(REPO_URL, Path::new("diem_repo")).await {
                    Ok(x) => x,
                    Err(e) => {
                        println!("{}", e);
                        continue;
                    }
                };
                println!("initialized");
                match metrics.start_analysis().await {
                    Ok(_) => println!("all good"),
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
