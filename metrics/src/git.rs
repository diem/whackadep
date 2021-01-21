use anyhow::{anyhow, Result};
use git2::Repository;
use std::path::Path;

const URL: &str = "https://github.com/diem/diem.git";

pub struct Repo(Repository);

impl Repo {
    // clone
    pub fn clone(url: &str, repo_folder: &Path) -> Result<Self> {
        match Repository::clone(url, repo_folder) {
            Ok(x) => Ok(Self(x)),
            Err(e) => Err(e.into()),
        }
    }

    // open an existing repository
    pub fn open(repo_folder: &Path) -> Result<Self> {
        match Repository::open(repo_folder) {
            Ok(x) => Ok(Self(x)),
            Err(e) => Err(e.into()),
        }
    }

    // performs a pull
    pub fn update(&mut self) -> Result<()> {
        // fetch
        self.0.find_remote("origin")?.fetch(&["main"], None, None);

        // get new head
        let fetch_head = self.0.find_reference("FETCH_HEAD")?;
        let fetch_commit = self.0.reference_to_annotated_commit(&fetch_head)?;
        let analysis = self.0.merge_analysis(&[&fetch_commit])?;
        if analysis.0.is_up_to_date() {
            Ok(())
        } else if analysis.0.is_fast_forward() {
            let refname = format!("refs/heads/{}", "main");
            let mut reference = self.0.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-Forward")?;
            self.0.set_head(&refname)?;
            self.0
                .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| e.into())
        } else {
            Err(anyhow!("Fast-forward only!"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn test_clone() {
        let dir = tempdir().unwrap();
        let repo = Repo::clone("https://github.com/mimoo/NoiseGo.git", dir.path()).unwrap();
    }

    #[test]
    fn test_update() {
        let dir = tempdir().unwrap();

        // init empty repo and add remote
        let output = Command::new("git")
            .current_dir(&dir)
            .arg("init")
            .output()
            .unwrap();
        println!("stdout: {:?}", String::from_utf8(output.stdout));
        println!("stderr: {:?}", String::from_utf8(output.stderr));

        let output = Command::new("git")
            .current_dir(&dir)
            .arg("remote")
            .arg("add")
            .arg("origin")
            .arg("https://github.com/mimoo/NoiseGo.git")
            .output()
            .unwrap();
        println!("stdout: {:?}", String::from_utf8(output.stdout));
        println!("stderr: {:?}", String::from_utf8(output.stderr));

        // pull
        let mut repo = Repo::open(dir.path()).expect("should be able to open");
        repo.update();
        //        let head = repo.0.head().unwrap();
        //        println!("{:?}", head.name());
        let statuses = repo.0.statuses(None).unwrap();
        for status in statuses.iter() {
            println!("{:?}", status.status());
        }

        // make sure we got latest
        // assert 467bb609019e8c11ba905200ade3320fb32ade9f
    }
}
