//! This module abstracts the usage of Git.
//! It is implemented by calling the `git` command-line tool directly.
//! Ideally this would be implemented with a library (for example, git2),
//! but ain't nobody got time for that!

use anyhow::Result;
use git2::Repository;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

pub struct Repo {
    pub repo_folder: PathBuf,
}

impl Repo {
    // open an existing repository
    pub fn new(repo_folder: &Path) -> Result<Self> {
        Repository::open(repo_folder)?;
        Ok(Self {
            repo_folder: repo_folder.to_path_buf(),
        })
    }

    // clone
    pub async fn clone(url: &str, repo_folder: &Path) -> Result<Self> {
        let output = Command::new("git")
            .args(&["clone", "--depth", "1", url])
            .arg(&repo_folder)
            .output()
            .await?;
        debug!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        Ok(Self {
            repo_folder: repo_folder.to_path_buf(),
        })
    }

    // performs a pull
    // TODO: since this might change the rust toolchain, do we want to do a rustup update here?
    pub async fn update(&self) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repo_folder)
            .arg("pull")
            .output()
            .await?;
        debug!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    }

    pub async fn head(&self) -> Result<String> {
        let output = Command::new("git")
            .current_dir(&self.repo_folder)
            .args(&["rev-parse", "HEAD"])
            .output()
            .await?;
        String::from_utf8(output.stdout).map_err(anyhow::Error::msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_t() {
        let dir = tempdir().unwrap();

        assert!(Repo::new(&dir.path()).is_err());

        Repo::clone("https://github.com/mimoo/disco.git", dir.path())
            .await
            .unwrap();

        assert!(Repo::new(dir.path()).is_ok());
    }
}
