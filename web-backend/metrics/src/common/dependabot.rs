//! This module abstract the [dependabot](https://github.com/dependabot/dependabot-core/) library.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::error;

#[derive(Deserialize, Default, Serialize, Debug, PartialEq, Clone)]
pub struct UpdateMetadata {
    changelog_url: Option<String>,
    // TODO: #[serde(skip)]
    changelog_text: Option<String>,
    commits_url: Option<String>,
    // TODO: #[serde(skip)]
    commits: Vec<Commit>,
}

#[derive(Deserialize, Default, Serialize, Debug, PartialEq, Clone)]
pub struct Commit {
    message: String,
    html_url: String,
}

pub async fn get_update_metadata(
    package_manager: &str,
    package: &str,
    version: &str,
    new_version: &str,
) -> Result<UpdateMetadata> {
    let mut dependabot_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dependabot_dir.push("dependabot");

    let output = Command::new("ruby")
        .current_dir(dependabot_dir)
        .env("DEPENDABOT_PACKAGE_MANAGER", package_manager)
        .env("DEPENDABOT_PACKAGE", package)
        .env("DEPENDABOT_VERSION", version)
        .env("DEPENDABOT_NEW_VERSION", new_version)
        .arg("changelog.rb")
        .output()
        .await?;

    if !output.status.success() {
        bail!(
            "couldn't run dependabot: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    serde_json::from_slice(&output.stdout).map_err(|e| {
        error!("{}", String::from_utf8_lossy(&output.stdout));
        anyhow::Error::msg(e)
    })
}
