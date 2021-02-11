use anyhow::{bail, Context, Result};
use guppy::graph::summaries::Summary;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tempfile::tempdir;
use tokio::process::Command;
use tracing::info;

pub struct CargoGuppy;

impl CargoGuppy {
    pub async fn run_cargo_guppy(repo_dir: &Path, out_dir: &Path) -> Result<()> {
        info!("running generate-summaries");

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
                "couldn't run cargo-guppy: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// deserialize the release summary
    pub fn parse_dependencies(path: &Path) -> Result<Summary> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(anyhow::Error::msg)
    }

    pub async fn fetch(repo_dir: &Path) -> Result<(Summary, Summary)> {
        let out_dir = tempdir()?;

        // 1. run cargo guppy
        Self::run_cargo_guppy(repo_dir, out_dir.path()).await?;

        // 2. deserialize the release and the full summary
        let path = out_dir.path().join("summary-release.json");
        let no_dev_summary = CargoGuppy::parse_dependencies(&path)
            .with_context(|| format!("couldn't open {:?}", path))?;

        let path = out_dir.path().join("summary-full.json"); // this will contain the dev dependencies
        let all_summary = CargoGuppy::parse_dependencies(&path)
            .with_context(|| format!("couldn't open {:?}", path))?;

        //
        Ok((no_dev_summary, all_summary))
    }
}
