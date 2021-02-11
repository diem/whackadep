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
use tracing::{debug, info};

//
// Internal structure
//

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Eq, Hash)]
/// A [RUSTSEC Advisory](https://rustsec.org/).
pub struct RustSec {
    /// yanked, unmaintained, etc.
    kind: String,
    /// The advisory information (id, description, date, etc.)
    advisory: Option<Advisory>,
    /// The versions patched and the versions unaffected.
    versions: Option<VersionInfo>,
}

//
// Structures to deserialize cargo-audit
//

#[derive(Deserialize)]
pub struct CargoAudit {
    vulnerabilities: Vulnerabilities,
    warnings: HashMap<String, Vec<Warning>>,
}

#[derive(Deserialize)]
struct Vulnerabilities {
    found: bool,
    list: Vec<Vulnerability>,
}

#[derive(Deserialize, Clone)]
struct Vulnerability {
    advisory: Advisory,
    versions: VersionInfo,
    package: PackageInfo,
}

#[derive(Deserialize, Clone)]
/// Warning can be "yanked", "unmaintained", ...
struct Warning {
    kind: String,
    package: PackageInfo,
    advisory: Option<Advisory>,    // null if "yanked"
    versions: Option<VersionInfo>, // null if "yanked"
}

#[derive(Deserialize, Clone)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Advisory {
    id: String,
    title: String,
    description: String,
    date: String,
    url: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
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
    ) -> Result<HashMap<(String, Version), Vec<RustSec>>> {
        // cargo audit --json
        let output = Command::new("cargo")
            .current_dir(repo_dir)
            .args(&["audit", "--json"])
            .output()
            .await?;

        // seems like cargo audit returns 0 and 1 when the stdout output is clean
        match output.status.code() {
            Some(0) => (),
            Some(1) => info!("vulnerability found!"),
            _ => bail!(
                "couldn't run cargo-audit, error code: {:?}, error: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ),
        };

        // load the json
        debug!("{:?}", String::from_utf8_lossy(&output.stdout));
        let audit: CargoAudit = serde_json::from_slice(&output.stdout)
            .map_err(anyhow::Error::msg)
            .context("Failed to deserialize cargo-audit output")?;

        // extract all the vulnerabilities
        let mut result: HashMap<(String, Version), Vec<RustSec>> = HashMap::new();

        if audit.vulnerabilities.found {
            for vulnerability in audit.vulnerabilities.list {
                let name = vulnerability.package.name.clone();
                let version = Version::parse(&vulnerability.package.version)?;
                let vuln = RustSec {
                    kind: "vulnerability".to_string(),
                    advisory: Some(vulnerability.advisory),
                    versions: Some(vulnerability.versions),
                };
                result
                    .entry((name, version))
                    .and_modify(|res| res.push(vuln.clone()))
                    .or_insert_with(|| {
                        let mut res = Vec::new();
                        res.push(vuln);
                        res
                    });
            }
        }

        // extract all the warnings
        for warning in audit.warnings.values().cloned().flatten() {
            let name = warning.package.name.clone();
            let version = Version::parse(&warning.package.version)?;
            let vuln = RustSec {
                kind: warning.kind,
                advisory: warning.advisory,
                versions: warning.versions,
            };
            result
                .entry((name, version))
                .and_modify(|res| res.push(vuln.clone()))
                .or_insert_with(|| {
                    let mut res = Vec::new();
                    res.push(vuln);
                    res
                });
        }

        //
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    #[ignore]
    async fn test_cargo_audit() {
        let mut repo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        repo_dir.push("../diem_repo");
        let res = CargoAudit::run_cargo_audit(repo_dir.as_path()).await;
        println!("{:?}", res);
    }
}
