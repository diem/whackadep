//! This module contains code to analyze Rust dependencies.
//!
//! # Stored structures
//!
//! Note that to remain backward compatible, these structures
//! should only be updated to add field, not remove.
//! (As deserialization of past data wouldn't work anymore.)
//! That being said, we might not store data for very long,
//! so this might not matter...
//!

use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use guppy_summaries::{PackageStatus, SummarySource, SummaryWithMetadata};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tempfile::tempdir;
use tracing::{debug, error, info};

pub mod cargoaudit;
pub mod cargoguppy;
pub mod cargotree;
pub mod cratesio;

use crate::common::dependabot::{self, UpdateMetadata};
use cargoaudit::CargoAudit;
use cargoguppy::CargoGuppy;

/// RustAnalysis contains the result of the analysis of a rust workspace
#[derive(Serialize, Deserialize, Default)]
pub struct RustAnalysis {
    /// Note that we do not use a map because the same dependency can be seen several times.
    /// This is due to different versions being used or/and being used directly and indirectly (transitively).
    dependencies: Vec<DependencyInfo>,
}

/// DependencyInfo contains the information obtained from a dependency.
/// Note that some fields might be filled in different stages (e.g. by the priority engine or the risk engine).
#[derive(Serialize, Deserialize)]
pub struct DependencyInfo {
    /// The name of the dependency.
    name: String,
    /// The current version of the dependency.
    version: Version,
    /// The repository where the dependency is hosted.
    repo: SummarySource,
    /// Is it a dev-dependency?
    dev: bool,
    /// Is it a direct, or a transitive dependency?
    direct: bool,
    /// An optional RUSTSEC advisory associated with the dependency and its version.
    rustsec: Option<RustSec>,
    /// An optional update available for the dependency.
    update: Option<Update>,
}

/// Update should contain any interesting information (red flags, etc.) about the changes observed in the new version
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Update {
    /// All versions
    // TODO: we're missing dates of creation for stats though..
    versions: Vec<Version>,
    /// changelog and commits between current version and last version available
    update_metadata: UpdateMetadata,
}

#[derive(Serialize, Deserialize)]
/// A [RUSTSEC Advisory](https://rustsec.org/).
pub struct RustSec {
    /// The advisory information (id, description, date, etc.)
    advisory: cargoaudit::Advisory,
    /// The versions patched and the versions unaffected.
    version_info: cargoaudit::VersionInfo,
}

impl RustAnalysis {
    /// The main function that will go over the flow:
    /// fetch -> filter -> updatables -> priority -> risk -> store
    pub async fn get_dependencies(repo_dir: &Path) -> Result<Self> {
        // 1. fetch
        info!("1. fetching dependencies...");
        let (all_deps, release_deps) = Self::fetch(repo_dir).await?;

        // 2. filter
        info!("2. filtering dependencies...");
        let mut rust_analysis = Self::filter(all_deps, release_deps)?;

        // 3. updatable
        info!("3. checking for updates...");
        rust_analysis.updatable().await?;

        // 4. priority
        info!("4. priority engine running...");
        rust_analysis.priority(repo_dir).await?;

        // 5. risk
        info!("5. risk engine running...");
        rust_analysis.risk()?;

        //
        Ok(rust_analysis)
    }

    /// 1. fetch
    async fn fetch(repo_dir: &Path) -> Result<(SummaryWithMetadata, SummaryWithMetadata)> {
        // 1. this will produce a json file containing no dev dependencies
        // (only transitive dependencies used in release)

        let out_dir = tempdir()?;
        debug!("tempdir: {:?}", out_dir);

        CargoGuppy::run_cargo_guppy(repo_dir, out_dir.path()).await?;

        // 2. deserialize the release and the full summary
        info!("deserialize result...");
        let path = out_dir.path().join("summary-release.json");
        let release_deps = CargoGuppy::parse_dependencies(&path)
            .with_context(|| format!("couldn't open {:?}", path))?;

        let path = out_dir.path().join("summary-full.json"); // this will contain the dev dependencies
        let all_deps = CargoGuppy::parse_dependencies(&path)
            .with_context(|| format!("couldn't open {:?}", path))?;

        //
        Ok((all_deps, release_deps))
    }

    /// 2. filter
    pub fn filter(
        all_deps: SummaryWithMetadata,
        release_deps: SummaryWithMetadata,
    ) -> Result<Self> {
        info!("filter result...");
        let mut dependencies = Vec::new();

        let all_deps_iter = all_deps
            .target_packages
            .iter()
            .chain(all_deps.host_packages.iter()); // "host" point to build-time dependencies

        for (summary_id, package_info) in all_deps_iter {
            // ignore workspace/internal packages
            if matches!(
                summary_id.source,
                SummarySource::Workspace { .. } | SummarySource::Path { .. }
            ) {
                continue;
            }
            if matches!(
                package_info.status,
                PackageStatus::Initial | PackageStatus::Workspace
            ) {
                continue;
            }

            // dev
            let dev = !release_deps.host_packages.contains_key(summary_id)
                && !release_deps.target_packages.contains_key(summary_id);

            // direct dependency?
            let direct = matches!(package_info.status, PackageStatus::Direct);

            // insert
            dependencies.push(DependencyInfo {
                name: summary_id.name.clone(),
                version: summary_id.version.clone(),
                repo: summary_id.source.clone(),
                update: None,
                dev: dev,
                direct: direct,
                rustsec: None,
            });
        }

        //
        Ok(Self { dependencies })
    }

    /// 3. Checks for updates in a set of crates
    async fn updatable(&mut self) -> Result<()> {
        // filter out non-crates.io dependencies
        let mut dependencies: Vec<String> = self
            .dependencies
            .iter()
            .filter(|dep| matches!(dep.repo, SummarySource::CratesIo))
            .map(|dep| dep.name.clone())
            .collect();

        // remove duplicates (assumption: the dependency list is sorted alphabetically)
        dependencies.dedup();

        // fetch versions for each dependency in that list
        let mut iterator = stream::iter(dependencies)
            .map(|dependency| async move {
                // get all versions for that dependency
                (
                    dependency.clone(),
                    cratesio::Crates::get_all_versions(&dependency).await,
                )
            })
            .buffer_unordered(10);

        // extract the result as a hashmap of name -> semver
        let mut dep_to_versions: HashMap<String, Vec<Version>> = HashMap::new();
        while let Some((dependency, crate_)) = iterator.next().await {
            if let Ok(crate_) = crate_ {
                let mut versions: Vec<Version> = crate_
                    .versions
                    .iter()
                    // parse as semver
                    .map(|version| Version::parse(&version.num))
                    .filter_map(Result::ok)
                    // TODO: log the error ^
                    .collect();
                versions.sort();
                dep_to_versions.insert(dependency, versions);
            }
        }

        // update our list of dependencies with that new information
        for dependency in &mut self.dependencies {
            let versions = dep_to_versions.get(dependency.name.as_str());
            if let Some(versions) = versions {
                // get GREAT versions
                // TODO: since the list is sorted, it should be faster to find the matching version and split_at there
                let greater_versions: Vec<Version> = versions
                    .iter()
                    .filter(|&version| version > &dependency.version)
                    .cloned()
                    .collect();

                // any update available?
                if greater_versions.len() > 0 {
                    let mut update = Update::default();
                    update.versions = greater_versions;
                    dependency.update = Some(update);
                }
            }
        }

        //
        Ok(())
    }

    /// 4. priority engine
    async fn priority(&mut self, repo_dir: &Path) -> Result<()> {
        // 1. get cargo-audit results
        info!("running cargo-audit");
        let audit = CargoAudit::run_cargo_audit(repo_dir).await?;
        for dependency in &mut self.dependencies {
            let res = audit.get(&(dependency.name.clone(), dependency.version.clone()));
            if let Some((advisory, version_info)) = res {
                dependency.rustsec = Some(RustSec {
                    advisory: advisory.clone(),
                    version_info: version_info.clone(),
                });
            }
        }

        // 2. fetch every changelog via dependabot
        if std::env::var("GITHUB_TOKEN").is_err()
            || std::env::var("GITHUB_TOKEN") == Ok("".to_string())
        {
            info!("skipping dependabot run due to GITHUB_TOKEN env var not found");
        } else {
            info!("running dependabot to get changelogs");
            let iterator = stream::iter(&mut self.dependencies)
                .map(|dependency| async move {
                    if let Some(update) = &mut dependency.update {
                        let new_version = match update.versions.last() {
                            Some(version) => version.to_string(),
                            None => {
                                error!(
                                    "couldn't find new version in a dependency update: {:?}",
                                    update
                                );
                                "".to_string()
                            }
                        };
                        let name = dependency.name.clone();
                        let version = dependency.version.to_string();
                        match dependabot::get_update_metadata(
                            "cargo",
                            &name,
                            &version,
                            &new_version,
                        )
                        .await
                        {
                            Ok(update_metadata) => update.update_metadata = update_metadata,
                            Err(e) => {
                                error!("couldn't get changelog for {}: {}", dependency.name, e)
                            }
                        };
                    }
                    ()
                })
                .buffer_unordered(10);
            iterator.collect::<()>().await;
        }

        //
        Ok(())
    }

    /// 5. risk engine
    fn risk(&mut self) -> Result<()> {
        Ok(())
    }
}
