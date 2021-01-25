use tokio::process::Command;
use anyhow::{Result, anyhow};

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
  pub async fn run_cargo_audit(repo_dir: &Path) -> Result<Self> {
    // make sure cargo-audit is installed 
    // this seems necessary because cargo-audit might have had an update, or because of the rust-toolchain?
    let output = Command::new("cargo")
          .current_dir(repo_dir)
          .args(&["install", "--force", "cargo-audit"])
          .output().await?;

    if !output.status.success() {
            return Err(anyhow!(
                "couldn't install cargo-audit: {:?}",
                String::from_utf8(output.stderr)
            ));
        }

    // cargo audit --json
      let output = Command::new("cargo")
          .current_dir(repo_dir)
          .args(&["audit", "--json"])
          .output().await?;

    if !output.status.success() {
      return Err(anyhow!(
          "couldn't run cargo-audit: {:?}",
          String::from_utf8(output.stderr)
      ));
    }

    // load the json
    serde_json::from_str(std.output)
  }
}