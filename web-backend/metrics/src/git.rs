use anyhow::{anyhow, Result};
use git2::Repository;
use std::path::{Path, PathBuf};
use std::process::Command;

// This module is implemented by calling the `git` command-line tool directly.
// Ideally this would be implemented with the git2 rust library,
// but we don't have time gosh damn it!

pub struct Repo {
    repo_folder: PathBuf,
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
    pub fn clone(url: &str, repo_folder: &Path) -> Result<Self> {
        let output = Command::new("git")
            .args(&["clone", url])
            .arg(&repo_folder)
            .output()?;
        println!("stdout: {:?}", String::from_utf8(output.stdout));
        Ok(Self {
            repo_folder: repo_folder.to_path_buf(),
        })
    }

    // performs a pull
    pub fn update(&self) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repo_folder)
            .arg("pull")
            .output()?;
        println!("stdout: {:?}", String::from_utf8(output.stdout));
        Ok(())
    }

    pub fn head(&self) -> Result<String> {
        let output = Command::new("git")
            .current_dir(&self.repo_folder)
            .args(&["rev-parse", "HEAD"])
            .output()?;
        String::from_utf8(output.stdout).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_t() {
        let dir = tempdir().unwrap();

        assert!(Repo::new(&dir.path()).is_err());

        Repo::clone("https://github.com/mimoo/disco.git", dir.path());

        assert!(Repo::new(dir.path()).is_ok());
    }
}
