use anyhow::{anyhow, Result};
use std::sync::mpsc::Receiver;

pub enum MetricsRequest {
    // request to refresh list of transitive dependencies
    Dependencies,
    // request to refresh
}

pub fn start(receiver: Receiver<MetricsRequest>) -> Result<()> {
    for request in receiver {
        match request {
            MetricsRequest::Dependencies => println!("received"),
            _ => return Err(anyhow!("invalid request")),
        };
    }
    Ok(())
}
