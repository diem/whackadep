use anyhow::{bail, Result};
use guppy_summaries::SummaryWithMetadata;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tokio::process::Command;

pub struct CargoGuppy;

impl CargoGuppy {
    pub async fn run_cargo_guppy(repo_dir: &Path, out_dir: &Path) -> Result<()> {
        println!("running generate-summaries");

        // 1. this will produce a json file containing no dev dependencies
        // (only transitive dependencies used in release)
        let output = Command::new("cargo")
            .current_dir(repo_dir)
            .args(&["x", "generate-summaries"])
            .arg(out_dir)
            .arg("json")
            .output()
            .await?;

        if !output.status.success() {
            bail!(
                "couldn't run cargo-guppy: {:?}",
                String::from_utf8(output.stderr)
            );
        }

        Ok(())
    }

    /// deserialize the release summary
    pub fn parse_dependencies(path: &Path) -> Result<SummaryWithMetadata> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(anyhow::Error::msg)
    }
}
