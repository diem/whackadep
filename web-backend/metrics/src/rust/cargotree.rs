use anyhow::{bail, Result};
use std::path::Path;
use tokio::process::Command;

pub struct CargoTree;

impl CargoTree {
    pub async fn init_cargo_tree() -> Result<()> {
        // make sure cargo-tree is installed
        // this seems necessary because cargo-audit might have had an update, or because of the rust-toolchain?
        let output = Command::new("cargo")
            .args(&["install", "cargo-tree"]) // TODO: use --force to force upgrade?
            .output()
            .await?;
        if !output.status.success() {
            bail!(
                "couldn't install cargo-tree: {:?}",
                String::from_utf8(output.stderr)
            );
        }
        Ok(())
    }

    pub async fn run_cargo_tree(
        repo_dir: &Path,
        package: String,
        version: String,
    ) -> Result<String> {
        let output = Command::new("cargo")
            .current_dir(repo_dir)
            .args(&["tree", "-i"]) // -i, --invert <SPEC>...          Invert the tree direction and focus on the given package
            .arg(&package)
            .output()
            .await?;
        if !output.status.success() {
            bail!(
                "couldn't run cargo-tree: {:?}",
                String::from_utf8(output.stderr)
            );
        }

        // convert stdout to string
        String::from_utf8(output.stdout).map_err(anyhow::Error::msg)
    }
}
