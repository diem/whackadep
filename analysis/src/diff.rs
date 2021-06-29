//! This module abstracts diff analysis between code versions

use anyhow::{anyhow, Result};
use guppy::graph::PackageMetadata;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{
    io::copy,
    path::{Path, PathBuf},
    fs::{read_dir, DirEntry, File},
};
use tempfile::{tempdir, TempDir};
use url::Url;
use git2::{
    AutotagOption, Delta, Diff, DiffOptions, Direction, FetchOptions, IndexAddOption, Oid,
    Repository, Signature, Tree,
};
use flate2::read::GzDecoder;
use tar::Archive;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CrateSourceDiffReport {
    // This type presents information on the difference
    // between crates.io source code
    // and git source hosted code
    // for a given version
    pub name: String,
    pub version: String,
    pub release_commit_found: Option<bool>,
    pub release_commit_analyzed: Option<bool>,
    pub is_different: Option<bool>,
    pub file_diff_stats: Option<FileDiffStats>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FileDiffStats {
    pub files_added: u64,
    pub files_modified: u64,
    pub files_deleted: u64,
}

pub struct DiffAnalyzer {
    dir: TempDir,   // hold temporary code files
    client: Client, // for downloading files
}

impl DiffAnalyzer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            dir: tempdir()?,
            client: Client::new(),
        })
    }

    pub fn analyze_crate_source_diff(
        self,
        package: &PackageMetadata,
    ) -> Result<CrateSourceDiffReport> {
        let name = package.name().to_string();
        let version = package.version().to_string();
        let repository = match package.repository() {
            Some(repo) => Self::trim_remote_url(repo)?,
            None => {
                return Ok(CrateSourceDiffReport {
                    name,
                    version,
                    ..Default::default()
                });
            }
        };

        //Setup a git repository for crates.io hosted source code
        let crate_source_path = self.get_cratesio_version(&name, &version)?;

        // Get commit for the version release in the git source
        let git_repo = self.get_git_repo(&name, &repository)?;

        Ok(CrateSourceDiffReport {
            ..Default::default()
        })
    }

    fn trim_remote_url(url: &str) -> Result<String> {
        // Trim down remote git urls like GitHub for cloning
        // in cases where the crate is in a subdirectory of the repo
        // in the format "host_url/owner/repo"
        let url = Url::from_str(url)?;

        let host = url.host_str().ok_or_else(|| anyhow!("invalid host"))?;
        // TODO: check if host is from recognized sources, e.g. github, bitbucket, gitlab

        let mut segments = url
            .path_segments()
            .ok_or_else(|| anyhow!("error parsing url"))?;
        let owner = segments
            .next()
            .ok_or_else(|| anyhow!("repository url missing owner"))?;
        let repo = segments
            .next()
            .map(|repo| repo.trim_end_matches(".git"))
            .ok_or_else(|| anyhow!("repository url missing repo"))?;

        let url = format!("https://{}/{}/{}", host, owner, repo);
        return Ok(url);
    }

    fn get_cratesio_version(&self, name: &str, version: &str) -> Result<PathBuf> {
        let download_path = format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            name, version
        );
        let dest_file = format!("{}-{}-cratesio", name, version);
        Ok(self.download_file(&download_path, &dest_file)?)
    }

    fn get_git_repo(&self, name: &str, url: &str) -> Result<Repository> {
        let dest_file = format!("{}-source", name);
        let dest_path = self.dir.path().join(&dest_file);
        let repo = Repository::clone(url, dest_path)?;
        Ok(repo)
    }

    fn download_file(&self, download_path: &str, dest_file: &str) -> Result<PathBuf> {
        // Destination directory to contain downloded files
        let dest_path = self.dir.path().join(&dest_file);

        // check if destination directory exists, if not proceed
        if !dest_path.exists() {
            // First download the file as tar_gz
            let targz_path = self.dir.path().join(format!("{}.targ.gz", dest_file));
            let mut targz_file = File::create(&targz_path)?;
            let mut response = self.client.get(download_path).send()?;
            copy(&mut response, &mut targz_file)?;

            // Then decompress the file
            self.decompress_targz(&targz_path, &dest_path)?;
        }

        // Get the only directory within dest_path where files are unpacked
        let entries: Vec<DirEntry> = read_dir(dest_path)?
            .filter_map(|entry| entry.ok())
            .collect();
        if entries.len() != 1 {
            return Err(anyhow!("Error in locating directory for unpacked files"));
        }

        // Return the directory containing unpacked files
        Ok(entries[0].path())
    }

    // note: in some functions, &self is not used,
    // however the functions may work with tempdirs set up by self,
    // therefore passing &self to them to make sure self (and, tempdir) still exists

    fn decompress_targz(&self, targz_path: &Path, dest_path: &Path) -> Result<()> {
        let tar_gz = File::open(targz_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(dest_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use guppy::{graph::PackageGraph, MetadataCommand};

    fn get_test_diff_analyzer() -> DiffAnalyzer {
        DiffAnalyzer::new().unwrap()
    }

    fn get_test_graph() -> PackageGraph {
        MetadataCommand::new()
            .current_dir(PathBuf::from("resources/test/valid_dep"))
            .build_graph()
            .unwrap()
    }

    #[test]
    fn test_diff_trim_git_url() {
        let url = "https://github.com/facebookincubator/cargo-guppy/tree/main/guppy";
        let trimmed_url = DiffAnalyzer::trim_remote_url(url).unwrap();
        assert_eq!(
            trimmed_url,
            "https://github.com/facebookincubator/cargo-guppy"
        );
    }

    #[test]
    fn test_diff_download_file() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "libc";
        let version = "0.2.97";
        let path = diff_analyzer
            .download_file(
                format!(
                    "https://crates.io//api/v1/crates/{}/{}/download",
                    name, version
                )
                .as_str(),
                format!("{}-{}", &name, &version).as_str(),
            )
            .unwrap();
        assert_eq!(path.exists(), true);
    }

    #[test]
    fn test_diff_crate_source() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "libc";
        let version = "0.2.97";
        let path = diff_analyzer.get_cratesio_version(&name, &version).unwrap();
        assert_eq!(path.exists(), true);
    }

    #[test]
    fn test_diff_git_repo() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "libc";
        let url = "https://github.com/rust-lang/libc";
        let repo = diff_analyzer.get_git_repo(&name, url).unwrap();
        assert_eq!(repo.workdir().is_none(), false);
        assert_eq!(repo.path().exists(), true);
        // TODO add tests for non-git repos
    }
}
