use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;

#[derive(Deserialize)]
struct CargoAudit {
    warnings: HashMap<String, Warning>,
}

#[derive(Deserialize)]
struct Warning {
    kind: String,
    package: PackageInfo,
    advisory: Advisory,
}

#[derive(Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Deserialize)]
struct Advisory {
    id: String,
    title: String,
    description: String,
    date: String,
    url: String,
}

impl CargoAudit {
    pub async fn init_cargo_audit() -> Result<()> {
        // make sure cargo-tree is installed
        // this seems necessary because cargo-audit might have had an update, or because of the rust-toolchain?
        let output = Command::new("cargo")
            .args(&["install", "--force", "cargo-tree"])
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow!(
                "couldn't install cargo-audit: {:?}",
                String::from_utf8(output.stderr)
            ));
        }
        Ok(())
    }

    pub async fn run_cargo_audit(repo_dir: &Path) -> Result<Self> {
        // cargo audit --json
        let output = Command::new("cargo")
            .current_dir(repo_dir)
            .args(&["audit", "--json"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!(
                "couldn't run cargo-audit: {:?}",
                String::from_utf8(output.stderr)
            ));
        }

        // load the json
        serde_json::from_slice(&output.stdout).map_err(anyhow::Error::msg)
    }
}
