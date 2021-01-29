
use tokio::process::Command;
use serde::Deserialize;
use anyhow::{Result, bail};

#[derive(Deserialize)]
pub struct UpdateMetadata {
  changelog_url: String,
  changelog_text: String,
  commits_url: String,
  commits_text: Vec<Commit>,
}

#[derive(Deserialize)]
pub struct Commit {
  message: String,
  html_url: String,
}

pub async fn get_changelog(package_manager: &str, package: &str, version: &str, new_version: &str) -> Result<UpdateMetadata> {
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
          "couldn't run dependabot: {:?}",
          String::from_utf8(output.stderr)
      );
  }

  serde_json::from_slice(&output.stdout).map_err(anyhow::Error::msg)
}