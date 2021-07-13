//! This module abstracts diff analysis between code versions

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use git2::{
    AutotagOption, Delta, Diff, DiffOptions, Direction, FetchOptions, IndexAddOption, Oid,
    Repository, Signature, Tree,
};
use guppy::{graph::PackageMetadata, MetadataCommand};
use regex::Regex;
use reqwest::blocking::Client;
use semver::Version;
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
use thiserror::Error;
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

#[derive(Debug, Error)]
#[error("Head commit not found in the repository for {crate_name}:{version}")]
pub struct HeadCommitNotFoundError {
    crate_name: String,
    version: Version,
}

pub(crate) struct VersionDiffInfo<'a> {
    pub repo: &'a Repository,
    pub commit_a: Oid,
    pub commit_b: Oid,
    pub diff: Diff<'a>,
}

pub(crate) fn trim_remote_url(url: &str) -> Result<String> {
    // Trim down remote git urls like GitHub for cloning
    // in cases where the crate is in a subdirectory of the repo
    // in the format "host_url/owner/repo"
    let url = Url::from_str(url)?;

    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("invalid host for {}", url))?;
    // TODO: check if host is from recognized sources, e.g. github, bitbucket, gitlab

    let mut segments = url
        .path_segments()
        .ok_or_else(|| anyhow!("error parsing url for {}", url))?;
    let owner = segments
        .next()
        .ok_or_else(|| anyhow!("repository url missing owner for {}", url))?;
    let repo = segments
        .next()
        .map(|repo| repo.trim_end_matches(".git"))
        .ok_or_else(|| anyhow!("repository url missing repo for {}", url))?;

    let url = format!("https://{}/{}/{}", host, owner, repo);
    Ok(url)
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
            Some(repo) => trim_remote_url(repo)?,
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
        let crate_repo = self.init_git(&crate_source_path)?;
        let crate_repo_head = crate_repo.head()?.peel_to_commit()?;
        let cratesio_tree = crate_repo_head.tree()?;

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

        // Add git repo as a remote to crate repo
        self.setup_remote(&crate_repo, &repository, &head_commit_oid.to_string())?;

        // At this point, crate_repo contains crate.io hosted source with a single commit
        //                and git source as a remote
        // Therefore, we can get diff between crate_repo master and any remote commit

        // Get release version commit within crate repo
        let git_version_commit = crate_repo.find_commit(head_commit_oid)?;
        let crate_git_tree = git_version_commit.tree()?;

        // Get the tree for the crate directory path
        // e.g., when a repository contains multiple crates
        git_repo.checkout_tree(
            git_repo.find_commit(head_commit_oid)?.tree()?.as_object(),
            None,
        )?;
        let toml_path = match self.locate_package_toml(&git_repo, &name) {
            Ok(path) => path,
            Err(_e) => {
                return Ok(CrateSourceDiffReport {
                    name,
                    version,
                    release_commit_found: Some(true),
                    release_commit_analyzed: Some(false),
                    ..Default::default()
                });
            }
        };
        let toml_path = toml_path
            .parent()
            .ok_or_else(|| anyhow!("Fatal: toml path returned as root"))?;
        let crate_git_tree =
            self.get_subdirectory_tree(&crate_repo, &crate_git_tree, &toml_path)?;

        let diff = crate_repo.diff_tree_to_tree(
            Some(&crate_git_tree),
            Some(&cratesio_tree),
            Some(&mut DiffOptions::new()),
        )?;

        // Uncomment while testing
        // self.display_diff(&diff)?;

        let file_diff_stats = self.get_crate_source_file_diff_report(&diff)?;

        Ok({
            CrateSourceDiffReport {
                name,
                version,
                release_commit_found: Some(true),
                release_commit_analyzed: Some(true),
                // Ignoring files from source not included in crates.io, possibly ignored
                is_different: Some(
                    file_diff_stats.files_added > 0 || file_diff_stats.files_modified > 0,
                ),
                file_diff_stats: Some(file_diff_stats),
            }
        })
    }

    fn get_cratesio_version(&self, name: &str, version: &str) -> Result<PathBuf> {
        let download_path = format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            name, version
        );
        let dest_file = format!("{}-{}-cratesio", name, version);
        self.download_file(&download_path, &dest_file)
    }

    pub fn get_git_repo(&self, name: &str, url: &str) -> Result<Repository> {
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

    fn init_git(&self, path: &Path) -> Result<Repository> {
        // initiates a git repository in the path
        let repo = Repository::init(path)?;

        // add and commit existing files
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        let oid = index.write_tree()?;
        let signature = Signature::now("user", "email@domain.com")?;
        let tree = repo.find_tree(oid)?;
        repo.commit(
            Some("HEAD"),     // point HEAD to new commit
            &signature,       // author
            &signature,       // committer
            "initial commit", // commit message
            &tree,            // tree
            &[],              // initial commit
        )?;

        Ok(Repository::open(path)?)
    }

    fn setup_remote(&self, repo: &Repository, url: &str, fetch_commit: &str) -> Result<()> {
        // Connect to remote
        let remote_name = "source";
        let mut remote = repo.remote(remote_name, url)?;
        remote.connect(Direction::Fetch)?;

        // Get default branch
        let default = remote.default_branch()?;
        let default = default
            .as_str()
            .ok_or_else(|| anyhow!("No default branch found"))?;

        // Fetch all tags
        let mut fetch_options = FetchOptions::new();
        fetch_options.download_tags(AutotagOption::All);

        // Fetch data
        remote.fetch(&[default, fetch_commit], Some(&mut fetch_options), None)?;

        Ok(())
    }

    fn locate_package_toml(&self, repo: &Repository, name: &str) -> Result<PathBuf> {
        // The repository may or may not contain multiple crates
        // Given a crate name and its repository
        // This function returns the path to Cargo.toml for the given crate

        let path = repo
            .path()
            .parent()
            .ok_or_else(|| anyhow!("Fatal: .git file has no parent"))?;

        // Possible error is that the past version of the given commit had different crate name
        // e.g., form_urlencoded 1.0.1 was percent encoding
        // Or, guppy failed to build a graph at the specfied commit,
        // https://github.com/facebookincubator/cargo-guppy/issues/416
        let graph = MetadataCommand::new().current_dir(path).build_graph()?;

        // Get crate path relative to the repository
        let member = graph.workspace().member_by_name(name)?;
        let toml_path = member.manifest_path();
        let path = toml_path.strip_prefix(path)?;

        Ok(PathBuf::from(path))
    }

    fn get_subdirectory_tree<'a>(
        &self,
        repo: &'a Repository,
        tree: &'a Tree,
        path: &Path,
    ) -> Result<Tree<'a>> {
        if path.file_name().is_none() {
            // Root of the repository path marked by an empty string
            return Ok(tree.clone());
        }
        let tree = tree.get_path(path)?.to_object(&repo)?.id();
        let tree = repo.find_tree(tree)?;
        Ok(tree)
    }

    // fn display_diff(&self, diff: &Diff) -> Result<()> {
    //     let stats = diff.stats()?;
    //     let mut format = git2::DiffStatsFormat::NONE;
    //     format |= git2::DiffStatsFormat::FULL;
    //     let buf = stats.to_buf(format, 80)?;
    //     print!(
    //         "difference between crates.io and source is:\n {}",
    //         std::str::from_utf8(&*buf).unwrap()
    //     );

    //     Ok(())
    // }

    fn get_crate_source_file_diff_report(&self, diff: &Diff) -> Result<FileDiffStats> {
        let mut files_added = 0;
        let mut files_modified = 0;
        let mut files_deleted = 0;

        // Ignore below files as they are changed whenever publishing to crates.io
        // TODO: compare Cargo.toml.orig in crates.io with Cargo.toml in git
        let ignore_paths: HashSet<PathBuf> = vec![
            ".cargo_vcs_info.json",
            "Cargo.toml",
            "Cargo.toml.orig",
            "Cargo.lock",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();

        for diff_delta in diff.deltas() {
            if ignore_paths.contains(
                diff_delta
                    .new_file()
                    .path()
                    .ok_or_else(|| anyhow!("no new file path for {:?}", diff_delta))?,
            ) {
                continue;
            }

            // TODO: Many times files like README are added/modified
            // by having only a single line in crates.io and deleting original contents
            // Also, we need to distinguish non source-code file here
            // to avoid noise in warning
            match diff_delta.status() {
                Delta::Added => {
                    files_added += diff_delta.nfiles() as u64;
                }
                Delta::Modified => {
                    // modification counts modified file as 2 files
                    files_modified += (diff_delta.nfiles() / 2) as u64;
                }
                Delta::Deleted => {
                    files_deleted += diff_delta.nfiles() as u64;
                }
                _ => (),
            }
        }

        Ok(FileDiffStats {
            files_added,
            files_modified,
            files_deleted,
        })
    }

    pub(crate) fn get_version_diff_info<'a>(
        &'a self,
        name: &str,
        repo: &'a Repository,
        version_a: &Version,
        version_b: &Version,
    ) -> Result<VersionDiffInfo<'a>> {
        // TODO: This function works only in cases where the root directory
        // of the git repository contains a Cargo.toml file
        let toml_path = self.locate_package_toml(&repo, &name)?;
        let toml_path = toml_path
            .parent()
            .ok_or_else(|| anyhow!("Cannot find crate directory"))?;

        let commit_oid_a = self
            .get_head_commit_oid_for_version(&repo, &name, &version_a.to_string())?
            .ok_or_else(|| HeadCommitNotFoundError {
                crate_name: name.to_string(),
                version: version_a.clone(),
            })?;
        let tree_a = repo.find_commit(commit_oid_a)?.tree()?;
        let tree_a = self.get_subdirectory_tree(&repo, &tree_a, &toml_path)?;

        let commit_oid_b = self
            .get_head_commit_oid_for_version(&repo, &name, &version_b.to_string())?
            .ok_or_else(|| HeadCommitNotFoundError {
                crate_name: name.to_string(),
                version: version_b.clone(),
            })?;
        let tree_b = repo.find_commit(commit_oid_b)?.tree()?;
        let tree_b = self.get_subdirectory_tree(&repo, &tree_b, &toml_path)?;

        let diff =
            repo.diff_tree_to_tree(Some(&tree_a), Some(&tree_b), Some(&mut DiffOptions::new()))?;

        Ok(VersionDiffInfo {
            repo,
            commit_a: commit_oid_a,
            commit_b: commit_oid_b,
            diff,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use guppy::{graph::PackageGraph, MetadataCommand};
    use serial_test::serial;

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
        let trimmed_url = trim_remote_url(url).unwrap();
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
        assert!(path.exists());
    }

    #[test]
    #[serial]
    fn test_diff_crate_source() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "libc";
        let version = "0.2.97";
        let path = diff_analyzer.get_cratesio_version(&name, &version).unwrap();
        assert!(path.exists());

        let repo = diff_analyzer.init_git(&path).unwrap();
        assert!(repo.path().exists());
        let commit = repo.head().unwrap().peel_to_commit();
        assert!(commit.is_ok());

        // Add git repo as a remote to crate repo
        let url = "https://github.com/rust-lang/libc";
        let fetch_commit = "1c66799b7b8b82269c6bff0eab97d1a30e37fd36";
        diff_analyzer
            .setup_remote(&repo, url, fetch_commit)
            .unwrap();
    }

    #[test]
    #[serial]
    fn test_diff_git_repo() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "libc";
        let url = "https://github.com/rust-lang/libc";
        let repo = diff_analyzer.get_git_repo(&name, url).unwrap();
        assert!(repo.workdir().is_some());
        assert!(repo.path().exists());
        // TODO add tests for non-git repos
    }

    #[test]
    #[serial]
    fn test_diff_head_commit_oid_for_version() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "test-version-tag";
        let url = "https://github.com/nasifimtiazohi/test-version-tag";

        let repo = diff_analyzer.get_git_repo(&name, url).unwrap();
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, &name, "0.0.8")
            .unwrap();
        assert!(oid.is_none());
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, &name, "10.0.8")
            .unwrap();
        assert_eq!(
            oid.unwrap(),
            Oid::from_str("51efd612af12183a682bb3242d41369d2879ad60").unwrap()
        );
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, &name, "10.0.8-")
            .unwrap();
        assert!(oid.is_none());

        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, "hakari", "0.3.0")
            .unwrap();
        assert_eq!(
            oid.unwrap(),
            Oid::from_str("946ddf053582067b843c19f1270fe92eaa0a7cb3").unwrap()
        );
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, "guppy", "0.3.0")
            .unwrap();
        assert_eq!(
            oid.unwrap(),
            Oid::from_str("dd7e5609e640f468a7e15a32fe36b607bae13e3e").unwrap()
        );
        let oid = diff_analyzer
            .get_head_commit_oid_for_version(&repo, "guppy-summaries", "0.3.0")
            .unwrap();
        assert_eq!(
            oid.unwrap(),
            Oid::from_str("24e00d39f90baa1daa2ef6f9a2bdb49e581874b3").unwrap()
        );
    }

    #[test]
    #[serial]
    fn test_diff_locate_cargo_toml() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "guppy";
        let url = "https://github.com/facebookincubator/cargo-guppy";
        let repo = diff_analyzer.get_git_repo(&name, url).unwrap();
        let path = diff_analyzer.locate_package_toml(&repo, name).unwrap();
        assert_eq!("guppy/Cargo.toml", path.to_str().unwrap());

        let diff_analyzer = get_test_diff_analyzer();
        let name = "octocrab";
        let url = "https://github.com/XAMPPRocky/octocrab";
        let repo = diff_analyzer.get_git_repo(&name, url).unwrap();
        let path = diff_analyzer.locate_package_toml(&repo, name).unwrap();
        assert_eq!("Cargo.toml", path.to_str().unwrap());
    }

    #[test]
    #[serial]
    fn test_diff_get_subdirectory_tree() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "guppy";
        let url = "https://github.com/facebookincubator/cargo-guppy";
        let repo = diff_analyzer.get_git_repo(&name, url).unwrap();
        let tree = repo
            .find_commit(Oid::from_str("dc6dcc151821e787ac02379bcd0319b26c962f55").unwrap())
            .unwrap()
            .tree()
            .unwrap();
        let path = PathBuf::from("guppy");
        let subdirectory_tree = diff_analyzer
            .get_subdirectory_tree(&repo, &tree, &path)
            .unwrap();
        assert_ne!(tree.id(), subdirectory_tree.id());
        // TODO: test that subdir tree doesn't have files from cargo-guppy
    }

    #[test]
    #[serial]
    fn test_diff_crate_source_diff_analyzer() {
        let graph = get_test_graph();
        for package in graph.packages() {
            if package.name() == "guppy" || package.name() == "octocrab" {
                println!("testing {}, {}", package.name(), package.version());
                let diff_analyzer = get_test_diff_analyzer();
                let report = diff_analyzer.analyze_crate_source_diff(&package).unwrap();
                if report.release_commit_found.is_none()
                    || !report.release_commit_found.unwrap()
                    || !report.release_commit_analyzed.unwrap()
                {
                    continue;
                }

                assert!(report.file_diff_stats.is_some());
                println!("{:?}", report);

                if package.name() == "guppy" {
                    assert!(!report.is_different.unwrap());
                }
                if package.name() == "octocrab" {
                    assert!(!report.is_different.unwrap());
                }
            }
        }
    }

    #[test]
    #[serial]
    fn test_diff_version_diff() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "guppy";
        let repository = "https://github.com/facebookincubator/cargo-guppy";

        let repo = diff_analyzer.get_git_repo(&name, &repository).unwrap();
        let version_diff_info = diff_analyzer
            .get_version_diff_info(
                name,
                &repo,
                &Version::parse("0.8.0").unwrap(),
                &Version::parse("0.9.0").unwrap(),
            )
            .unwrap();

        assert_eq!(
            version_diff_info.commit_a,
            Oid::from_str("dc6dcc151821e787ac02379bcd0319b26c962f55").unwrap()
        );
        assert_eq!(
            version_diff_info.commit_b,
            Oid::from_str("fe61a8b85feab1963ee1985bf0e4791fdd354aa5").unwrap()
        );

        let diff = version_diff_info.diff;
        assert_eq!(diff.stats().unwrap().files_changed(), 6);
        assert_eq!(diff.stats().unwrap().insertions(), 199);
        assert_eq!(diff.stats().unwrap().deletions(), 82);
    }

    #[test]
    #[serial]
    fn test_diff_head_commit_not_found_error() {
        let diff_analyzer = get_test_diff_analyzer();
        let name = "guppy";
        let repository = "https://github.com/facebookincubator/cargo-guppy";

        let repo = diff_analyzer.get_git_repo(&name, &repository).unwrap();
        let diff = diff_analyzer
            .get_version_diff_info(
                name,
                &repo,
                &Version::parse("0.0.0").unwrap(),
                &Version::parse("0.9.0").unwrap(),
            )
            .map_err(|error| {
                error
                    .root_cause()
                    .downcast_ref::<HeadCommitNotFoundError>()
                    // If not the error type, downcast will be None
                    .is_none()
            })
            .err()
            .unwrap();
        assert!(!diff);
    }
}
