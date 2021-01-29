//! There are different types of results in Cargo-audit
//! The important distinction is:
//! - there is no patch
//! - there are versions that are unaffected

use anyhow::{bail, Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;
use tracing::debug;

//
// Structures to deserialize cargo-audit
//

#[derive(Deserialize)]
pub struct CargoAudit {
    warnings: HashMap<String, Vec<Warning>>,
}

#[derive(Deserialize, Clone)]
struct Warning {
    kind: String,
    package: PackageInfo,
    advisory: Advisory,
    versions: VersionInfo,
}

#[derive(Deserialize, Clone)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Advisory {
    id: String,
    title: String,
    description: String,
    date: String,
    url: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct VersionInfo {
    patched: Vec<String>,
    unaffected: Vec<String>,
}

//
// Logic
//

impl CargoAudit {
    pub async fn init_cargo_audit() -> Result<()> {
        // make sure cargo-tree is installed
        // this seems necessary because cargo-audit might have had an update, or because of the rust-toolchain?
        let output = Command::new("cargo")
            .args(&["install", "cargo-audit"]) // TODO: use --force to force upgrade?
            .output()
            .await?;
        if !output.status.success() {
            bail!(
                "couldn't install cargo-audit: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    pub async fn run_cargo_audit(
        repo_dir: &Path,
    ) -> Result<HashMap<(String, Version), (Advisory, VersionInfo)>> {
        // cargo audit --json
        let output = Command::new("cargo")
            .current_dir(repo_dir)
            .args(&["audit", "--json"])
            .output()
            .await?;

        if !output.status.success() {
            bail!(
                "couldn't run cargo-audit: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // load the json
        debug!("{:?}", String::from_utf8_lossy(&output.stdout));
        let audit: CargoAudit = serde_json::from_slice(&output.stdout)
            .map_err(anyhow::Error::msg)
            .context("Failed to deserialize cargo-audit output")?;

        // sort all the warnings into dependency -> Advisory
        let warnings: Vec<Warning> = audit.warnings.values().cloned().flatten().collect();

        let advisories: HashMap<(String, Version), (Advisory, VersionInfo)> = warnings
            .into_iter()
            .map(|warning| {
                if let Ok(version) = Version::parse(&warning.package.version) {
                    Some((
                        (warning.package.name, version),
                        (warning.advisory, warning.versions),
                    ))
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        //
        Ok(advisories)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_cargo_audit() {
        let mut repo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        repo_dir.push("../diem_repo");
        let res = CargoAudit::run_cargo_audit(repo_dir.as_path()).await;
        println!("{:?}", res);
    }
}
