// to get more metrics we need to get a diff of the changes
// so we can get things like:
// - LOC
// - build.rs was changed
// - the change introduces new dependencies

use anyhow::{bail, ensure, Result};
use regex::Regex;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use tokio::process::Command;
use tracing::info;

async fn download_cargo_crate(crate_with_version: &str, extract_dir: &Path) -> Result<()> {
    // return path to downloaded crate
    // cargo download cargo-download==0.1.2
    let extract_path = extract_dir.join(crate_with_version);
    let extract_path = extract_path.as_path();
    fs::create_dir_all(extract_path)?;
    let output = Command::new("cargo")
        .current_dir(extract_dir)
        .args(&["download", "-x", "-o"])
        .arg(extract_path)
        .arg(crate_with_version)
        .output()
        .await?;

    ensure!(
        output.status.success(),
        "Couldn't run cargo download: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

async fn diff_cargo_crates(
    path_to_original_crate: &Path,
    path_to_new_crate: &Path,
) -> Result<bool> {
    let diff_output = Command::new("git")
        .args(&["diff", "--no-index", "--name-only"])
        .arg(path_to_original_crate)
        .arg(path_to_new_crate)
        .output()
        .await?;

    // returns '1' if no difference found, '0' if difference found
    if !matches!(diff_output.status.code(), Some(1) | Some(0)) {
        bail!(
            "Error running git diff command: {}",
            String::from_utf8_lossy(&diff_output.stderr)
        );
    }

    // TODO: for now, we hardcode build.rs
    // but we need to parse Cargo.toml in all directories and identify
    // custom build.rs files
    // see: https://doc.rust-lang.org/cargo/reference/manifest.html#the-build-field
    //TODO: optimize the regex with lazy_static (https://docs.rs/regex/1.4.3/regex/index.html#example-avoid-compiling-the-same-regex-in-a-loop)
    let pattern = Regex::new(r"(?m)\bbuild\.rs\b")
        .expect("create regex pattern, should work with no problems");
    Ok(pattern.is_match(&String::from_utf8(diff_output.stdout)?))
}

pub async fn init_cargo_download() -> Result<()> {
    //! install cargo-download crate
    info!("Installing cargo-download crate");
    let output = Command::new("cargo")
        .args(&["install", "cargo-download"])
        .output()
        .await?;
    ensure!(
        output.status.success(),
        "couldn't install cargo-download: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

pub async fn is_diff_in_buildrs(
    cargo_crate_original_version: &str,
    cargo_crate_new_version: &str,
) -> Result<bool> {
    //! Download two versions of a crate and returns boolean if build.rs changed between the two versions

    let out_dir = tempdir()?;
    let out_dir = out_dir.path();

    download_cargo_crate(cargo_crate_original_version, &out_dir).await?;
    download_cargo_crate(cargo_crate_new_version, &out_dir).await?;

    let original_crate = out_dir.join(cargo_crate_original_version);
    let original_crate = original_crate.as_path();

    let latest_crate = out_dir.join(cargo_crate_new_version);
    let latest_crate = latest_crate.as_path();

    diff_cargo_crates(original_crate, latest_crate).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_download_cargo_crate() {
        let out_dir = tempdir().unwrap();
        let out_dir = out_dir.path();
        download_cargo_crate("cargo-download==0.1.2", &out_dir).await.unwrap();
        assert!(out_dir.join("cargo-download==0.1.2").exists());
    }

    #[tokio::test]
    async fn test_diff_cargo_crates() {
        let out_dir = tempdir().unwrap();
        let out_dir = out_dir.path();
        // tiny-keccak-2.0.0 does not have build.rs
        // tiny-keccak-2.0.1 does have build.rs
        // tiny-keccak-2.0.2 has diff from 2.0.1
        download_cargo_crate("tiny-keccak==2.0.0", &out_dir)
            .await
            .unwrap();
        download_cargo_crate("tiny-keccak==2.0.1", &out_dir)
            .await
            .unwrap();
        download_cargo_crate("tiny-keccak==2.0.2", &out_dir)
            .await
            .unwrap();

        let t_k_0 = out_dir.join("tiny-keccak==2.0.0");
        let t_k_0 = t_k_0.as_path();

        let t_k_1 = out_dir.join("tiny-keccak==2.0.1");
        let t_k_1 = t_k_1.as_path();

        let t_k_2 = out_dir.join("tiny-keccak==2.0.2");
        let t_k_2 = t_k_2.as_path();

        assert_eq!(diff_cargo_crates(t_k_0, t_k_0).await.unwrap(), false);
        assert!(diff_cargo_crates(t_k_0, t_k_1).await.unwrap());
        assert!(diff_cargo_crates(t_k_1, t_k_2).await.unwrap());
    }

    #[tokio::test]
    async fn test_is_diff_in_buildrs() {
        assert!(
            is_diff_in_buildrs("tiny-keccak==2.0.0", "tiny-keccak==2.0.1")
                .await
                .unwrap()
        );
        assert!(
            is_diff_in_buildrs("tiny-keccak==2.0.1", "tiny-keccak==2.0.2")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_init_cargo_download() {
        assert!(init_cargo_download().await.is_ok());
    }
}
