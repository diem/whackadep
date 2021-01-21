use anyhow::Result;
use std::sync::mpsc::Receiver;

pub fn start(receiver: Receiver<String>) -> Result<()> {
    for request in receiver {
        println!("received: {:?}", request);
    }
    Ok(())
}
