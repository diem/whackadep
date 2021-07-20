//! This module abstracts analyses for dependency update review.

use crate::cratesio::CratesioAnalyzer;
use anyhow::{anyhow, Result};
use geiger::RsFileMetrics;
use git2::{build::CheckoutBuilder, Delta, Diff};
use guppy::graph::{
    cargo::{CargoOptions, CargoResolverVersion},
    feature::{FeatureFilter, StandardFeatures},
    summaries::{
        diff::{SummaryDiff, SummaryDiffStatus},
        Summary, SummaryId,
    },
    BuildTargetId, PackageGraph, PackageMetadata,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ops::Sub,
    path::PathBuf,
};
use url::Url;

use crate::advisory::AdvisoryLookup;
use crate::diff::{CrateSourceDiffReport, DiffAnalyzer, HeadCommitNotFoundError, VersionDiffInfo};

#[derive(Debug, Clone)]
pub enum DependencyType {
    Host,
    Target,
}

#[derive(Debug, Clone)]
pub struct DependencyChangeInfo {
    pub name: String,
    pub repository: Option<String>,
    pub dep_type: DependencyType,
    // TODO: accomodate to specify source and commits here
    // e.g., crate_a updating from commit_a to commit_b from repo_a
    pub old_version: Option<Version>, // None when a dep is added
    pub new_version: Option<Version>, // None when a dep is removed
    // Build script paths for the dependency
    pub build_script_paths: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateReviewReport {
    pub dep_update_review_reports: Vec<DepUpdateReviewReport>,
    pub version_conflicts: Vec<VersionConflict>,
}

#[derive(Debug, Clone)]
pub struct DepUpdateReviewReport {
    pub name: String,
    pub prior_version: VersionInfo,
    pub updated_version: VersionInfo,
    pub diff_stats: Option<VersionDiffStats>,
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub name: String,
    pub version: Version,
    pub downloads: u64,
    pub crate_source_diff_report: CrateSourceDiffReport,
    pub known_advisories: Vec<CrateVersionRustSecAdvisory>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrateVersionRustSecAdvisory {
    pub id: String,
    pub title: String,
    pub url: Option<Url>,
}

pub struct VersionChangeInfo {
    pub old_version: Option<Version>, // None when a dep is added
    pub new_version: Option<Version>, // None when a dep is removed
}

#[derive(Debug, Clone)]
pub struct VersionDiffStats {
    pub files_changed: u64,
    pub rust_files_changed: u64,
    pub insertions: u64,
    pub deletions: u64,
    pub modified_build_scripts: HashSet<String>, // Empty indicates no change in build scripts
    pub unsafe_file_changed: Vec<FileUnsafeChangeStats>,
}

#[derive(Debug, Clone)]
pub enum VersionConflict {
    // Case 1: A dep has two copies of different version
    //         as a direct and a transitive dep in the graph
    DirectTransitiveVersionConflict {
        name: String,
        direct_dep_version: Version,
        transitive_dep_version: Version,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileUnsafeCodeChangeStatus {
    UnsafeCounterModified, // when we have a delta in unsafe counter
    NoUnsafeCode,          // changed file(s) contained no unsafe code before and after change
    AllUnsafeCodeRemoved,  // there was unsafe code before the change that all got removed
    Uncertain,             // changed files contain unsafe code,
                           // TODO: our tool isn't that smart yet to verify if unsafe code lines have been changed
}

#[derive(Debug, Clone)]
pub struct FileUnsafeChangeStats {
    pub file: String,
    pub change_type: Delta,
    pub unsafe_change_status: FileUnsafeCodeChangeStatus,
    pub unsafe_delta: UnsafeDelta, // Delta in Unsafe counter:
    // Unsafe Delta cannot detect the case where a line is modified
    // in which case the unsafe counter before and after will be the same
    // TODO: detect if unsafe code has been modified in a diff

    // Below field indicate the post state of an added/modified file
    // and will be None in case of a deleted file
    pub unsafe_status: Option<RsFileMetrics>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct UnsafeDelta {
    pub functions: i64,
    pub expressions: i64,
    pub impls: i64,
    pub traits: i64,
    pub methods: i64,
}

impl UnsafeDelta {
    pub fn has_no_change(&self) -> bool {
        self.expressions == 0
            && self.functions == 0
            && self.impls == 0
            && self.traits == 0
            && self.methods == 0
    }
}

impl Sub for UnsafeDelta {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            functions: self.functions - rhs.functions,
            expressions: self.expressions - rhs.expressions,
            impls: self.impls - rhs.impls,
            traits: self.traits - rhs.traits,
            methods: self.methods - rhs.methods,
        }
    }
}

pub struct UpdateAnalyzer {
    // the key will be crate name, old version, and updated version
    cache: RefCell<HashMap<(String, Version, Version), DepUpdateReviewReport>>,
}

impl UpdateAnalyzer {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }
    pub fn analyze_updates(
        self,
        prior_graph: &PackageGraph,
        post_graph: &PackageGraph,
    ) -> Result<UpdateReviewReport> {
        // Analyzing with default options
        self.analyze_updates_with_options(
            prior_graph,
            post_graph,
            &Self::get_default_cargo_options(),
            StandardFeatures::All,
        )
    }

    pub fn analyze_updates_with_options<'a>(
        self,
        prior_graph: &'a PackageGraph,
        post_graph: &'a PackageGraph,
        cargo_opts: &CargoOptions,
        feature_filter: impl FeatureFilter<'a>,
    ) -> Result<UpdateReviewReport> {
        // Get the changed dependency stats
        let dep_change_infos =
            Self::compare_pacakge_graphs(&prior_graph, &post_graph, cargo_opts, feature_filter)?;

        // Filter version updates
        let updated_deps: Vec<DependencyChangeInfo> = dep_change_infos
            .iter()
            .filter(
                |dep| match (dep.old_version.as_ref(), dep.new_version.as_ref()) {
                    (Some(old), Some(new)) => new > old,
                    _ => false,
                },
            )
            .cloned()
            .collect();
        // TODO: add reporting for version downgrades, add, and remove

        // clean cache if there's anything in a weird scenario
        // And store all the distinct update review in the cache
        self.cache.borrow_mut().clear();
        for dep in &updated_deps {
            self.get_update_review(dep)?;
        }
        let dep_update_review_reports: Vec<DepUpdateReviewReport> =
            self.cache.borrow_mut().drain().map(|(_k, v)| v).collect();

        let version_conflicts: Vec<VersionConflict> =
            Self::determine_version_conflict(&dep_change_infos, &post_graph);

        Ok(UpdateReviewReport {
            dep_update_review_reports,
            version_conflicts,
        })
    }

    fn determine_version_conflict(
        dep_change_infos: &[DependencyChangeInfo],
        graph: &PackageGraph,
    ) -> Vec<VersionConflict> {
        let mut conflicts: Vec<VersionConflict> = Vec::new();

        // Check for direct-transitive version conflict
        let direct_dependencies = Self::get_direct_dependencies(&graph);
        for dep_change_info in dep_change_infos {
            if let (Some(package), Some(new_version)) = (
                direct_dependencies
                    .iter()
                    .find(|dep| dep.name() == dep_change_info.name),
                dep_change_info.new_version.clone(),
            ) {
                if *package.version() != new_version {
                    conflicts.push(VersionConflict::DirectTransitiveVersionConflict {
                        name: package.name().to_string(),
                        direct_dep_version: package.version().clone(),
                        transitive_dep_version: new_version,
                    })
                }
            }
        }

        conflicts
    }

    fn get_direct_dependencies(graph: &PackageGraph) -> Vec<PackageMetadata> {
        graph
            .query_workspace()
            .resolve_with_fn(|_, link| {
                let (from, to) = link.endpoints();
                from.in_workspace() && !to.in_workspace()
            })
            .packages(guppy::graph::DependencyDirection::Forward)
            .filter(|pkg| !pkg.in_workspace())
            .collect()
    }

    fn get_default_cargo_options() -> CargoOptions<'static> {
        let mut cargo_opts = CargoOptions::new();
        cargo_opts.set_version(CargoResolverVersion::V2);
        cargo_opts.set_include_dev(true);
        cargo_opts
    }

    fn compare_pacakge_graphs<'a>(
        prior_graph: &'a PackageGraph,
        post_graph: &'a PackageGraph,
        cargo_opts: &CargoOptions,
        mut feature_filter: impl FeatureFilter<'a>,
    ) -> Result<Vec<DependencyChangeInfo>> {
        let prior_summary = Self::get_summary(&prior_graph, &mut feature_filter, &cargo_opts)?;
        let post_summary = Self::get_summary(&post_graph, &mut feature_filter, &cargo_opts)?;
        let diff = SummaryDiff::new(&prior_summary, &post_summary);

        let mut dep_change_infos: Vec<DependencyChangeInfo> = Vec::new();

        for (summary_id, summary_diff_status) in diff.host_packages.changed.iter() {
            dep_change_infos.push(Self::get_dependency_change_info(
                prior_graph,
                post_graph,
                &summary_id,
                &summary_diff_status,
                DependencyType::Host,
            )?);
        }

        for (summary_id, summary_diff_status) in diff.target_packages.changed.iter() {
            dep_change_infos.push(Self::get_dependency_change_info(
                &prior_graph,
                &post_graph,
                &summary_id,
                &summary_diff_status,
                DependencyType::Target,
            )?);
        }

        Ok(dep_change_infos)
    }

    fn get_summary<'a>(
        graph: &'a PackageGraph,
        feature_filter: impl FeatureFilter<'a>,
        cargo_opts: &CargoOptions,
    ) -> Result<Summary> {
        let summary = graph
            .resolve_all()
            .to_feature_set(feature_filter)
            .into_cargo_set(cargo_opts)?
            .to_summary(cargo_opts)?;
        Ok(summary)
    }

    fn get_dependency_change_info(
        prior_graph: &PackageGraph,
        post_graph: &PackageGraph,
        summary_id: &SummaryId,
        summary_diff_status: &SummaryDiffStatus,
        dep_type: DependencyType,
    ) -> Result<DependencyChangeInfo> {
        let name = summary_id.name.clone();
        let version_change_info =
            Self::get_version_change_info_from_summarydiff(&summary_id, &summary_diff_status);
        let repository = Self::get_repository_from_graphs(&[prior_graph, post_graph], &name);

        let mut build_script_paths: HashSet<String> = HashSet::new();
        let old_version = version_change_info.old_version;
        if old_version.is_some() {
            Self::get_build_script_paths(prior_graph, &name)?
                .into_iter()
                .for_each(|x| {
                    build_script_paths.insert(x);
                });
        }
        let new_version = version_change_info.new_version;
        if new_version.is_some() {
            Self::get_build_script_paths(post_graph, &name)?
                .into_iter()
                .for_each(|x| {
                    build_script_paths.insert(x);
                });
        }

        Ok(DependencyChangeInfo {
            name,
            repository,
            dep_type,
            old_version,
            new_version,
            build_script_paths,
        })
    }

    fn get_build_script_paths(graph: &PackageGraph, crate_name: &str) -> Result<HashSet<String>> {
        let package = graph
            .packages()
            .find(|p| p.name() == crate_name)
            .ok_or_else(|| anyhow!("crate not present in package graph"))?;

        let package_path = package
            .manifest_path()
            .parent()
            .ok_or_else(|| anyhow!("invalid Cargo.toml path"))?;

        let build_script_paths: Result<HashSet<String>> = package
            .build_targets()
            .filter(|b| b.id() == BuildTargetId::BuildScript)
            .map(|b| Ok(b.path().strip_prefix(package_path)?.as_str().to_string()))
            .collect();

        build_script_paths
    }

    fn get_version_change_info_from_summarydiff(
        summary_id: &SummaryId,
        summary_diff_status: &SummaryDiffStatus,
    ) -> VersionChangeInfo {
        let mut old_version: Option<Version> = None;
        let mut new_version: Option<Version> = None;

        match summary_diff_status {
            SummaryDiffStatus::Added { .. } => {
                new_version = Some(summary_id.version.clone());
            }
            SummaryDiffStatus::Modified {
                old_version: version,
                ..
            } => {
                new_version = Some(summary_id.version.clone());
                if version.is_some() {
                    old_version = Some(version.unwrap().clone());
                }
            }
            SummaryDiffStatus::Removed { .. } => {
                old_version = Some(summary_id.version.clone());
            }
        }

        VersionChangeInfo {
            old_version,
            new_version,
        }
    }

    fn get_repository_from_graphs(graphs: &[&PackageGraph], crate_name: &str) -> Option<String> {
        graphs
            .iter()
            .find_map(|g| Self::get_repository_from_graph(g, crate_name))
    }

    fn get_repository_from_graph(graph: &PackageGraph, crate_name: &str) -> Option<String> {
        let package = graph.packages().find(|p| p.name() == crate_name)?;
        let repository = package.repository()?.to_string();
        Some(repository)
    }

    fn get_update_review(
        &self,
        dep_change_info: &DependencyChangeInfo,
    ) -> Result<DepUpdateReviewReport> {
        if let (Some(old_version), Some(new_version)) = (
            dep_change_info.old_version.as_ref(),
            dep_change_info.new_version.as_ref(),
        ) {
            if new_version < old_version {
                return Err(anyhow!("dependency change is a downgrade - not update "));
            }

            let name = &dep_change_info.name;
            let key = (name.clone(), old_version.clone(), new_version.clone());

            if let Some(report) = self.get_update_review_report_from_cache(&key) {
                return Ok(report);
            }

            let cratesio_analyzer = CratesioAnalyzer::new()?;
            let advisory_lookup = AdvisoryLookup::new()?;

            let prior_version = VersionInfo {
                name: name.clone(),
                version: old_version.clone(),
                downloads: cratesio_analyzer.get_version_downloads(&name, &old_version)?,
                crate_source_diff_report: DiffAnalyzer::new()?.analyze_crate_source_diff(
                    name,
                    &old_version.to_string(),
                    dep_change_info.repository.as_deref(),
                )?,
                known_advisories: advisory_lookup
                    .get_crate_version_advisories(&name, &old_version.to_string())?
                    .iter()
                    .filter(|advisory| advisory.metadata.withdrawn.is_none())
                    .map(|advisory| Self::get_crate_version_rustsec_advisory(advisory))
                    .collect(),
            };

            let updated_version = VersionInfo {
                name: name.clone(),
                version: new_version.clone(),
                downloads: cratesio_analyzer.get_version_downloads(&name, &new_version)?,
                crate_source_diff_report: DiffAnalyzer::new()?.analyze_crate_source_diff(
                    name,
                    &new_version.to_string(),
                    dep_change_info.repository.as_deref(),
                )?,
                known_advisories: advisory_lookup
                    .get_crate_version_advisories(&name, &new_version.to_string())?
                    .iter()
                    .filter(|advisory| advisory.metadata.withdrawn.is_none())
                    .map(|advisory| Self::get_crate_version_rustsec_advisory(advisory))
                    .collect(),
            };

            let diff_stats = Self::analyze_version_diff(&dep_change_info)?;

            let report = DepUpdateReviewReport {
                name: dep_change_info.name.clone(),
                prior_version,
                updated_version,
                diff_stats,
            };
            self.cache.borrow_mut().insert(key.clone(), report);
            self.get_update_review_report_from_cache(&key)
                .ok_or_else(|| anyhow!("fatal cache error for update analyzer"))
        } else {
            Err(anyhow!(
                "dependency change is either an addition or removal - not update"
            ))
        }
    }

    fn get_crate_version_rustsec_advisory(
        advisory: &rustsec::advisory::Advisory,
    ) -> CrateVersionRustSecAdvisory {
        CrateVersionRustSecAdvisory {
            id: advisory.id().as_str().to_string(),
            title: advisory.metadata.title.clone(),
            url: advisory.metadata.url.clone(),
        }
    }

    fn analyze_version_diff(
        dep_change_info: &DependencyChangeInfo,
    ) -> Result<Option<VersionDiffStats>> {
        if let (name, Some(old_version), Some(new_version)) = (
            &dep_change_info.name,
            &dep_change_info.old_version,
            &dep_change_info.new_version,
        ) {
            let diff_analyzer = DiffAnalyzer::new()?;

            if let (Ok(repo_old_version), Ok(repo_new_version)) = (
                diff_analyzer.get_git_repo_for_cratesio_version(&name, &old_version.to_string()),
                diff_analyzer.get_git_repo_for_cratesio_version(&name, &new_version.to_string()),
            ) {
                // Get version diff info from crates.io if avalaiable on crates.io
                let version_diff_info = diff_analyzer
                    .get_version_diff_info_between_repos(&repo_old_version, &repo_new_version)?;
                Ok(Some(Self::get_version_diff_stats(
                    &dep_change_info,
                    &version_diff_info,
                )?))
            } else if let Some(repository) = &dep_change_info.repository {
                // Get version diff info from git source if avaialbe
                let repo = diff_analyzer.get_git_repo(&name, &repository)?;
                let version_diff_info = match diff_analyzer.get_version_diff_info(
                    &name,
                    &repo,
                    &old_version,
                    &new_version,
                ) {
                    Ok(info) => info,
                    Err(error) => {
                        match error.root_cause().downcast_ref::<HeadCommitNotFoundError>() {
                            Some(_err) => return Ok(None),
                            None => return Err(anyhow!("fatal error in fetching head commit")),
                        }
                    }
                };
                Ok(Some(Self::get_version_diff_stats(
                    &dep_change_info,
                    &version_diff_info,
                )?))
            } else {
                Ok(None)
            }
        } else {
            // If old version, or new version is none, there is no update diff
            Ok(None)
        }
    }

    fn get_version_diff_stats(
        dep_change_info: &DependencyChangeInfo,
        version_diff_info: &VersionDiffInfo,
    ) -> Result<VersionDiffStats> {
        let stats = version_diff_info.diff.stats()?;

        let modified_build_scripts: HashSet<String> = dep_change_info
            .build_script_paths
            .iter()
            .filter(|path| Self::is_file_modified(&path, &version_diff_info.diff))
            .map(|path| path.to_string())
            .collect();

        let files_unsafe_change_stats = Self::analyze_unsafe_changes_in_diff(&version_diff_info)?;

        Ok(VersionDiffStats {
            files_changed: stats.files_changed() as u64,
            rust_files_changed: files_unsafe_change_stats.len() as u64,
            insertions: stats.insertions() as u64,
            deletions: stats.deletions() as u64,
            modified_build_scripts,
            unsafe_file_changed: files_unsafe_change_stats
                .into_iter()
                .filter(|report| {
                    report.unsafe_change_status != FileUnsafeCodeChangeStatus::NoUnsafeCode
                })
                .collect(),
        })
    }

    fn is_file_modified(path: &str, diff: &Diff) -> bool {
        let mut modified_file_paths: HashSet<&str> = HashSet::new();

        for diff_delta in diff.deltas() {
            let path: Option<&str> = diff_delta.old_file().path().and_then(|path| path.to_str());
            if let Some(p) = path {
                modified_file_paths.insert(p);
            }

            let path: Option<&str> = diff_delta.new_file().path().and_then(|path| path.to_str());
            if let Some(p) = path {
                modified_file_paths.insert(p);
            }
        }

        modified_file_paths.contains(path)
    }

    fn get_file_unsafe_change_status(
        rs_file_metrics: &Option<RsFileMetrics>,
        unsafe_delta: &UnsafeDelta,
    ) -> FileUnsafeCodeChangeStatus {
        if let Some(rs_file_metrics) = rs_file_metrics {
            match (
                unsafe_delta.has_no_change(),
                rs_file_metrics.counters.has_unsafe(),
            ) {
                (true, true) => FileUnsafeCodeChangeStatus::Uncertain,
                (true, false) => FileUnsafeCodeChangeStatus::NoUnsafeCode,
                (false, true) => FileUnsafeCodeChangeStatus::UnsafeCounterModified,
                (false, false) => FileUnsafeCodeChangeStatus::AllUnsafeCodeRemoved,
            }
        } else {
            // File deleted
            match unsafe_delta.has_no_change() {
                true => FileUnsafeCodeChangeStatus::NoUnsafeCode,
                false => FileUnsafeCodeChangeStatus::AllUnsafeCodeRemoved,
            }
        }
    }

    fn analyze_unsafe_changes_in_diff(
        version_diff_info: &VersionDiffInfo,
    ) -> Result<Vec<FileUnsafeChangeStats>> {
        let repo_path = version_diff_info
            .repo
            .path()
            .parent()
            .ok_or_else(|| anyhow!("error evaluating local repository path"))?;

        let mut checkout_builder = CheckoutBuilder::new();
        checkout_builder.force();

        // Checkout repo at prior commit and get unsafe stats for diff files
        let mut old_files_unsafe_stats: HashMap<PathBuf, Option<RsFileMetrics>> = HashMap::new();
        version_diff_info.repo.checkout_tree(
            &version_diff_info
                .repo
                .find_object(version_diff_info.commit_a, None)?,
            Some(&mut checkout_builder),
        )?;
        for diff_delta in version_diff_info.diff.deltas() {
            if let Some(path) = diff_delta.old_file().path() {
                let old_file_unsafe_stats = geiger::find::find_unsafe_in_file(
                    &repo_path.join(path),
                    geiger::IncludeTests::No,
                )
                .ok();
                old_files_unsafe_stats.insert(path.to_path_buf(), old_file_unsafe_stats);
            }
        }

        // Checkout repo at post commit and get unsafe stats for diff files
        let mut new_files_unsafe_stats: HashMap<PathBuf, Option<RsFileMetrics>> = HashMap::new();
        version_diff_info.repo.checkout_tree(
            &version_diff_info
                .repo
                .find_object(version_diff_info.commit_b, None)?,
            Some(&mut checkout_builder),
        )?;
        for diff_delta in version_diff_info.diff.deltas() {
            if let Some(path) = diff_delta.new_file().path() {
                let new_file_unsafe_stats = geiger::find::find_unsafe_in_file(
                    &repo_path.join(path),
                    geiger::IncludeTests::No,
                )
                .ok();
                new_files_unsafe_stats.insert(path.to_path_buf(), new_file_unsafe_stats);
            }
        }

        // Calculate changes in unsafe counter for each file
        let mut files_unsafe_change_stats: Vec<FileUnsafeChangeStats> = Vec::new();
        for diff_delta in version_diff_info.diff.deltas() {
            let old_file_unsafe_stats = diff_delta
                .old_file()
                .path()
                .and_then(|path| old_files_unsafe_stats.get(path))
                .and_then(|path| path.clone());
            let new_file_unsafe_stats = diff_delta
                .new_file()
                .path()
                .and_then(|path| new_files_unsafe_stats.get(path))
                .and_then(|path| path.clone());

            if old_file_unsafe_stats.is_none() && new_file_unsafe_stats.is_none() {
                // Not a rust file
                continue;
            }

            let unsafe_delta = Self::get_unsafe_delta_from_rs_file_metrics(&new_file_unsafe_stats)
                - Self::get_unsafe_delta_from_rs_file_metrics(&old_file_unsafe_stats);
            let unsafe_status = new_file_unsafe_stats;
            files_unsafe_change_stats.push(FileUnsafeChangeStats {
                file: diff_delta
                    .new_file()
                    .path()
                    .or_else(|| diff_delta.old_file().path())
                    .and_then(|path| path.to_str())
                    .ok_or_else(|| anyhow!("fatal error: diff contains no files"))?
                    .to_string(),
                change_type: diff_delta.status(),
                // while unsafe_change_status can be computed from the rest of the two fields,
                // it makes sure the caller would not have to worry about this
                unsafe_change_status: Self::get_file_unsafe_change_status(
                    &unsafe_status,
                    &unsafe_delta,
                ),
                unsafe_delta,
                unsafe_status,
            })
        }

        Ok(files_unsafe_change_stats)
    }

    fn get_unsafe_delta_from_rs_file_metrics(
        rs_file_metrics: &Option<RsFileMetrics>,
    ) -> UnsafeDelta {
        match rs_file_metrics {
            Some(rfm) => UnsafeDelta {
                functions: rfm.counters.functions.unsafe_ as i64,
                expressions: rfm.counters.exprs.unsafe_ as i64,
                impls: rfm.counters.item_impls.unsafe_ as i64,
                traits: rfm.counters.item_traits.unsafe_ as i64,
                methods: rfm.counters.methods.unsafe_ as i64,
            },
            None => UnsafeDelta::default(),
        }
    }

    fn get_update_review_report_from_cache(
        &self,
        key: &(String, Version, Version),
    ) -> Option<DepUpdateReviewReport> {
        self.cache.borrow().get(key).cloned()
    }
}

#[cfg(test)]
mod test {
    use super::{
        DependencyType, DiffAnalyzer, FileUnsafeCodeChangeStatus, PackageGraph, StandardFeatures,
        UpdateAnalyzer, VersionConflict::DirectTransitiveVersionConflict,
    };
    use crate::diff::trim_remote_url;
    use guppy::{CargoMetadata, MetadataCommand};
    use semver::Version;
    use serial_test::serial;
    use std::path::PathBuf;

    struct PackageGraphPair {
        prior: PackageGraph,
        post: PackageGraph,
    }

    fn get_test_graph_pair_guppy() -> PackageGraphPair {
        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/prior_guppy_change_metadata.json"
        ))
        .unwrap();
        let prior = metadata.build_graph().unwrap();

        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/post_guppy_change_metadata.json"
        ))
        .unwrap();
        let post = metadata.build_graph().unwrap();

        PackageGraphPair { prior, post }
    }

    fn get_test_graph_pair_libc() -> PackageGraphPair {
        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/prior_libc_change_metadata.json"
        ))
        .unwrap();
        let prior = metadata.build_graph().unwrap();

        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/post_libc_change_metadata.json"
        ))
        .unwrap();
        let post = metadata.build_graph().unwrap();

        PackageGraphPair { prior, post }
    }

    fn get_test_graph_pair_conflict() -> PackageGraphPair {
        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/prior_conflict_metadata.json"
        ))
        .unwrap();
        let prior = metadata.build_graph().unwrap();

        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/post_conflict_metadata.json"
        ))
        .unwrap();
        let post = metadata.build_graph().unwrap();

        PackageGraphPair { prior, post }
    }

    fn get_test_graph_pair_rustsec() -> PackageGraphPair {
        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/prior_rustsec_metadata.json"
        ))
        .unwrap();
        let prior = metadata.build_graph().unwrap();

        let metadata =
            CargoMetadata::parse_json(include_str!("../resources/test/post_rustsec_metadata.json"))
                .unwrap();
        let post = metadata.build_graph().unwrap();

        PackageGraphPair { prior, post }
    }

    fn get_test_update_analyzer() -> UpdateAnalyzer {
        UpdateAnalyzer::new()
    }

    #[test]
    fn test_update_compare_package_graph() {
        let package_graph_pair = get_test_graph_pair_guppy();

        let dep_change_infos = UpdateAnalyzer::compare_pacakge_graphs(
            &package_graph_pair.prior,
            &package_graph_pair.post,
            &UpdateAnalyzer::get_default_cargo_options(),
            StandardFeatures::All,
        )
        .unwrap();

        // Total changes
        assert_eq!(20, dep_change_infos.len());

        // Host deps
        assert_eq!(
            5,
            dep_change_infos
                .iter()
                .filter(|dep| matches!(dep.dep_type, DependencyType::Host))
                .count()
        );

        // Target deps
        assert_eq!(
            15,
            dep_change_infos
                .iter()
                .filter(|dep| matches!(dep.dep_type, DependencyType::Target))
                .count()
        );

        // Deps added
        assert_eq!(
            8,
            dep_change_infos
                .iter()
                .filter(|dep| dep.old_version.is_none() && dep.new_version.is_some())
                .count()
        );

        // Deps removed
        assert_eq!(
            10,
            dep_change_infos
                .iter()
                .filter(|dep| dep.old_version.is_some() && dep.new_version.is_none())
                .count()
        );

        // Deps version changed
        assert_eq!(
            2,
            dep_change_infos
                .iter()
                .filter(|dep| dep.old_version.is_some() && dep.new_version.is_some())
                .count()
        );
    }

    #[test]
    fn test_update_get_repository_from_graphs() {
        let package_graph_pair = get_test_graph_pair_guppy();
        let graphs = vec![&package_graph_pair.prior, &package_graph_pair.post];

        assert_eq!(
            "https://github.com/facebookincubator/cargo-guppy",
            trim_remote_url(&UpdateAnalyzer::get_repository_from_graphs(&graphs, "guppy").unwrap())
                .unwrap()
        );

        assert_eq!(
            "https://github.com/rust-lang/git2-rs",
            trim_remote_url(&UpdateAnalyzer::get_repository_from_graphs(&graphs, "git2").unwrap())
                .unwrap()
        );

        assert_eq!(
            "https://github.com/XAMPPRocky/octocrab",
            trim_remote_url(
                &UpdateAnalyzer::get_repository_from_graphs(&graphs, "octocrab").unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    #[serial]
    fn test_update_review_report_guppy() {
        let package_graph_pair = get_test_graph_pair_guppy();
        let update_analyzer = get_test_update_analyzer();
        let update_review_reports = update_analyzer
            .analyze_updates(&package_graph_pair.prior, &package_graph_pair.post)
            .unwrap();
        assert_eq!(update_review_reports.dep_update_review_reports.len(), 2);
        for report in &update_review_reports.dep_update_review_reports {
            if report.name == "guppy" {
                assert_eq!(
                    report.prior_version.version,
                    Version::parse("0.8.0").unwrap()
                );
                assert_eq!(
                    report.updated_version.version,
                    Version::parse("0.9.0").unwrap()
                );
                assert_eq!(report.diff_stats.as_ref().unwrap().files_changed, 9);
                assert_eq!(report.diff_stats.as_ref().unwrap().rust_files_changed, 4);
                assert_eq!(report.diff_stats.as_ref().unwrap().insertions, 244);
                assert_eq!(report.diff_stats.as_ref().unwrap().deletions, 179);
                assert!(report
                    .diff_stats
                    .as_ref()
                    .unwrap()
                    .modified_build_scripts
                    .is_empty());
                assert_eq!(
                    report
                        .diff_stats
                        .as_ref()
                        .unwrap()
                        .unsafe_file_changed
                        .len(),
                    0
                );
            }
        }
        println!("{:?}", update_review_reports);
    }

    #[test]
    #[serial]
    fn test_update_review_report_libc() {
        let package_graph_pair = get_test_graph_pair_libc();
        let update_analyzer = get_test_update_analyzer();
        let update_review_reports = update_analyzer
            .analyze_updates(&package_graph_pair.prior, &package_graph_pair.post)
            .unwrap();
        assert_eq!(update_review_reports.dep_update_review_reports.len(), 1);
        let report = update_review_reports
            .dep_update_review_reports
            .get(0)
            .unwrap();
        assert_eq!(report.prior_version.name, report.name);
        assert_eq!(report.prior_version.name, report.updated_version.name);
        assert_eq!(
            report.prior_version.version,
            Version::parse("0.2.92").unwrap()
        );
        assert_eq!(
            report.updated_version.version,
            Version::parse("0.2.93").unwrap()
        );
        // downloads for old 0.2.92 and 0.2.93 with an order of magnitude of difference
        // to be the exact same is very low, equal stats is likely a bug
        assert_ne!(
            report.prior_version.downloads,
            report.updated_version.downloads
        );
        assert_eq!(report.diff_stats.as_ref().unwrap().files_changed, 78);
        assert_eq!(report.diff_stats.as_ref().unwrap().rust_files_changed, 73);
        assert_eq!(report.diff_stats.as_ref().unwrap().insertions, 1333);
        assert_eq!(report.diff_stats.as_ref().unwrap().deletions, 4942);
        assert_eq!(
            report
                .diff_stats
                .as_ref()
                .unwrap()
                .modified_build_scripts
                .len(),
            1
        );
        assert_eq!(
            report
                .diff_stats
                .as_ref()
                .unwrap()
                .unsafe_file_changed
                .len(),
            12
        );
    }

    #[test]
    fn test_update_build_script_paths() {
        let graph = MetadataCommand::new()
            .current_dir(PathBuf::from("resources/test/valid_dep"))
            .build_graph()
            .unwrap();

        let build_script_paths = UpdateAnalyzer::get_build_script_paths(&graph, "libc").unwrap();
        assert_eq!(build_script_paths.len(), 1);
        assert_eq!(build_script_paths.iter().next().unwrap(), "build.rs");

        let graph = MetadataCommand::new()
            .current_dir(PathBuf::from("resources/test/valid_dep"))
            .build_graph()
            .unwrap();
        let build_script_paths =
            UpdateAnalyzer::get_build_script_paths(&graph, "valid_dep").unwrap();
        // TODO: there is another file build/custom_build.rs that is called
        // from the build script which won't be available from guppy
        // we need to add functionaliyt for that.
        assert_eq!(build_script_paths.len(), 1);
        assert_eq!(build_script_paths.iter().next().unwrap(), "build/main.rs");
    }

    #[test]
    #[serial]
    fn test_update_build_script_change() {
        let package_graph_pair = get_test_graph_pair_libc();
        let update_analyzer = get_test_update_analyzer();
        let update_review_reports = update_analyzer
            .analyze_updates(&package_graph_pair.prior, &package_graph_pair.post)
            .unwrap();
        let report = update_review_reports
            .dep_update_review_reports
            .iter()
            .find(|report| report.name == "libc")
            .unwrap();
        let build_scripts = &report.diff_stats.as_ref().unwrap().modified_build_scripts;
        assert_eq!(build_scripts.len(), 1);
        assert_eq!(build_scripts.iter().next().unwrap(), "build.rs");

        let package_graph_pair = get_test_graph_pair_guppy();
        let update_analyzer = get_test_update_analyzer();
        let update_review_reports = update_analyzer
            .analyze_updates(&package_graph_pair.prior, &package_graph_pair.post)
            .unwrap();
        let report = update_review_reports
            .dep_update_review_reports
            .iter()
            .find(|report| report.name == "guppy")
            .unwrap();
        let build_scripts = &report.diff_stats.as_ref().unwrap().modified_build_scripts;
        assert_eq!(build_scripts.len(), 0);
    }

    #[test]
    fn test_update_version_conflict() {
        let package_graph_pair = get_test_graph_pair_conflict();
        let dep_change_infos = UpdateAnalyzer::compare_pacakge_graphs(
            &package_graph_pair.prior,
            &package_graph_pair.post,
            &UpdateAnalyzer::get_default_cargo_options(),
            StandardFeatures::All,
        )
        .unwrap();

        let version_conflicts =
            UpdateAnalyzer::determine_version_conflict(&dep_change_infos, &package_graph_pair.post);
        assert_eq!(version_conflicts.len(), 1);

        let conflict = version_conflicts.get(0).unwrap();
        match conflict {
            DirectTransitiveVersionConflict { name, .. } => {
                assert_eq!(name, "target-spec");
            }
        }
    }

    #[test]
    #[serial]
    fn test_update_rustsec() {
        let package_graph_pair = get_test_graph_pair_rustsec();
        let update_analyzer = get_test_update_analyzer();
        let reports = update_analyzer
            .analyze_updates(&package_graph_pair.prior, &package_graph_pair.post)
            .unwrap();
        let report = reports
            .dep_update_review_reports
            .iter()
            .find(|report| report.name == "tokio")
            .unwrap();

        assert!(report
            .prior_version
            .known_advisories
            .iter()
            .any(|adv| adv.id == "RUSTSEC-2021-0072"));
        assert!(!report
            .updated_version
            .known_advisories
            .iter()
            .any(|adv| adv.id == "RUSTSEC-2021-0072"));
    }

    #[test]
    #[serial]
    fn test_update_geiger_file_scanning() {
        let name = "test_unsafe";
        let repository = "https://github.com/nasifimtiazohi/test-version-tag";
        let diff_analyzer = DiffAnalyzer::new().unwrap();
        let repo = diff_analyzer.get_git_repo(&name, &repository).unwrap();

        let version_diff_info = diff_analyzer
            .get_version_diff_info(
                &name,
                &repo,
                &Version::parse("2.0.0").unwrap(),
                &Version::parse("2.1.0").unwrap(),
            )
            .unwrap();
        let files_unsafe_change_stats =
            UpdateAnalyzer::analyze_unsafe_changes_in_diff(&version_diff_info).unwrap();
        let file = files_unsafe_change_stats
            .iter()
            .find(|stat| stat.file == "src/main.rs")
            .unwrap();
        assert_eq!(file.unsafe_delta.functions, 1);
        assert_eq!(file.unsafe_delta.methods, 1);
        assert_eq!(file.unsafe_delta.traits, 1);
        assert_eq!(file.unsafe_delta.impls, 0);
        assert_eq!(file.unsafe_delta.expressions, 4);
        let file = files_unsafe_change_stats
            .iter()
            .find(|stat| stat.file == "src/newanother.rs")
            .unwrap();
        assert_eq!(file.unsafe_delta.functions, 0);
        assert_eq!(file.unsafe_delta.methods, 0);
        assert_eq!(file.unsafe_delta.traits, 0);
        assert_eq!(file.unsafe_delta.impls, 0);
        assert_eq!(file.unsafe_delta.expressions, 0);

        let version_diff_info = diff_analyzer
            .get_version_diff_info(
                &name,
                &repo,
                &Version::parse("2.1.0").unwrap(),
                &Version::parse("2.4.0").unwrap(),
            )
            .unwrap();
        let files_unsafe_change_stats =
            UpdateAnalyzer::analyze_unsafe_changes_in_diff(&version_diff_info).unwrap();
        println!("{:?}", files_unsafe_change_stats);

        let file = files_unsafe_change_stats
            .iter()
            .find(|stat| stat.file == "src/main.rs")
            .unwrap();
        assert_eq!(file.unsafe_delta.functions, -1);
        assert_eq!(file.unsafe_delta.methods, -1);
        assert_eq!(file.unsafe_delta.traits, -1);
        assert_eq!(file.unsafe_delta.impls, 0);
        assert_eq!(file.unsafe_delta.expressions, -2);
        let file = files_unsafe_change_stats
            .iter()
            .find(|stat| stat.file == "src/newanother.rs")
            .unwrap();
        assert_eq!(file.unsafe_delta.functions, 1);
        assert_eq!(file.unsafe_delta.methods, 1);
        assert_eq!(file.unsafe_delta.traits, 1);
        assert_eq!(file.unsafe_delta.impls, 0);
        assert_eq!(file.unsafe_delta.expressions, 2);

        let version_diff_info = diff_analyzer
            .get_version_diff_info(
                &name,
                &repo,
                &Version::parse("2.4.0").unwrap(),
                &Version::parse("2.5.0").unwrap(),
            )
            .unwrap();
        let files_unsafe_change_stats =
            UpdateAnalyzer::analyze_unsafe_changes_in_diff(&version_diff_info).unwrap();
        println!("{:?}", files_unsafe_change_stats);

        let file = files_unsafe_change_stats
            .iter()
            .find(|stat| stat.file == "src/main.rs")
            .unwrap();
        // A line has changes withing unsafe block
        // but the total counter remains same
        // TODO: how to detect such changes?
        assert_eq!(file.unsafe_delta.expressions, 0);
    }

    #[test]
    #[serial]
    fn test_update_unsafe_change_status() {
        let name = "test_unsafe";
        let repository = "https://github.com/nasifimtiazohi/test-version-tag";
        let diff_analyzer = DiffAnalyzer::new().unwrap();
        let repo = diff_analyzer.get_git_repo(&name, &repository).unwrap();

        let version_diff_info = diff_analyzer
            .get_version_diff_info(
                &name,
                &repo,
                &Version::parse("2.6.0").unwrap(),
                &Version::parse("3.1.0").unwrap(),
            )
            .unwrap();
        let files_unsafe_change_stats =
            UpdateAnalyzer::analyze_unsafe_changes_in_diff(&version_diff_info).unwrap();

        println!("{:?}", files_unsafe_change_stats);

        for report in &files_unsafe_change_stats {
            if report.file == "src/main.rs" {
                assert_eq!(
                    report.unsafe_change_status,
                    FileUnsafeCodeChangeStatus::Uncertain
                );
            }
            if report.file == "src/newanother.rs" {
                assert_eq!(
                    report.unsafe_change_status,
                    FileUnsafeCodeChangeStatus::UnsafeCounterModified
                );
            }
            if report.file == "src/unsafefiletoremove.rs" {
                assert_eq!(
                    report.unsafe_change_status,
                    FileUnsafeCodeChangeStatus::AllUnsafeCodeRemoved
                );
            }
            if report.file == "src/unsafetoremove.rs" {
                assert_eq!(
                    report.unsafe_change_status,
                    FileUnsafeCodeChangeStatus::AllUnsafeCodeRemoved
                );
            }
            if report.file == "src/nounsafe.rs" {
                assert_eq!(
                    report.unsafe_change_status,
                    FileUnsafeCodeChangeStatus::NoUnsafeCode
                );
            }
        }
    }
}
