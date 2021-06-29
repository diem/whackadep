//! This module abstracts diff analysis between code versions

use anyhow::{anyhow, Result};
use guppy::graph::PackageMetadata;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{
    io::copy,
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};
use url::Url;

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
}
