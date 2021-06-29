//! This module abstracts diff analysis between code versions

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use git2::{
    AutotagOption, Delta, Diff, DiffOptions, Direction, FetchOptions, IndexAddOption, Oid,
    Repository, Signature, Tree,
};
use guppy::graph::PackageMetadata;
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{
    collections::{HashMap, HashSet},
    fs::{read_dir, DirEntry, File},
    io::copy,
    path::{Path, PathBuf},
};
use tar::Archive;
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

        //Setup a git repository for crates.io hosted source code
        let crate_source_path = self.get_cratesio_version(&name, &version)?;

        // Get commit for the version release in the git source
        let git_repo = self.get_git_repo(&name, &repository)?;
        let head_commit_oid =
            match self.get_head_commit_oid_for_version(&git_repo, &name, &version)? {
                Some(commit) => commit,
                None => {
                    return Ok(CrateSourceDiffReport {
                        name,
                        version,
                        release_commit_found: Some(false),
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

    fn get_head_commit_oid_for_version(
        &self,
        repo: &Repository,
        name: &str,
        version: &str,
    ) -> Result<Option<Oid>> {
        // Get candidate tags with a heuristic that tag will end with the version string
        let pattern = format!("*{}", version);
        let candidate_tags = repo.tag_names(Some(&pattern))?;

        let mut hm: HashMap<&str, Oid> = HashMap::new();
        for tag in candidate_tags.iter() {
            let tag = tag.ok_or_else(|| anyhow!("Error in fetching tags"))?;
            let commit = repo.revparse_single(tag)?.peel_to_commit()?;
            hm.insert(tag, commit.id());
        }

        // Now we check through a series of heuristics if tag matches a version
        let version_formatted_for_regex = version.replace(".", "\\.");
        let patterns = [
            // 1. Ensure the version part does not follow any digit between 1-9,
            // e.g., to distinguish betn 0.1.8 vs 10.1.8
            format!(r"^(?:.*[^1-9])?{}$", version_formatted_for_regex),
            // 2. If still more than one candidate,
            // check the extistence of crate name
            format!(r"^.*{}(?:.*[^1-9])?{}$", name, version_formatted_for_regex),
            // 3. check if  and only if crate name and version string is present
            // besides non-alphanumeric, e.g., to distinguish guppy vs guppy-summaries
            format!(r"^.*{}\W*{}$", name, version_formatted_for_regex),
        ];

        for pattern in &patterns {
            let re = Regex::new(&pattern)?;

            // drain filter hashmap if tag matches the pattern
            let mut candidate_tags: Vec<&str> = Vec::new();
            for (tag, _oid) in hm.iter() {
                if !re.is_match(tag) {
                    candidate_tags.push(tag);
                }
            }
            for tag in candidate_tags {
                hm.remove(tag);
            }

            // multiple tags can point to the same commit
            let unique_commits: HashSet<Oid> = hm.values().cloned().collect();
            if unique_commits.len() == 1 {
                return Ok(Some(*unique_commits.iter().next().unwrap()));
            }
        }

        // TODO: add checking of changes in Cargo.toml file for a deterministic evaluation

        // If still failed to determine a single commit hash, return None
        Ok(None)
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

    #[test]
    fn test_diff_head_commit_oid_for_version() {
        let diff_analyzer = get_test_diff_analyzer();

        let name = "tomcat";
        let url = "https://github.com/apache/tomcat";
        let repo = diff_analyzer.get_git_repo(&name, url).unwrap();
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, &name, "0.0.8")
            .unwrap();
        assert_eq!(oid.is_none(), true);
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, &name, "10.0.8")
            .unwrap();
        assert_eq!(
            oid.unwrap(),
            Oid::from_str("64520a63e23437b4e92db42bfc70a20d1f9e79c4").unwrap()
        );
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, &name, "10.0.8-")
            .unwrap();
        assert_eq!(oid.is_none(), true);

        let name = "cargo-guppy";
        let url = "https://github.com/facebookincubator/cargo-guppy";
        let repo = diff_analyzer.get_git_repo(&name, url).unwrap();
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, "hakari", "0.3.0")
            .unwrap();
        assert_eq!(
            oid.unwrap(),
            Oid::from_str("fe61a8b85feab1963ee1985bf0e4791fdd354aa5").unwrap()
        );
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, "guppy", "0.3.0")
            .unwrap();
        assert_eq!(
            oid.unwrap(),
            Oid::from_str("9fd47f429f7453938279ecbe8b3f1dd077d655fa").unwrap()
        );
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, "guppy-summaries", "0.3.0")
            .unwrap();
        assert_eq!(
            oid.unwrap(),
            Oid::from_str("7a2c65e6f9fbcd008b240d8574fe7057291caa06").unwrap()
        );
    }
}
