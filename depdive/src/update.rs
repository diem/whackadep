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
use crate::diff::{DiffAnalyzer, HeadCommitNotFoundError, VersionDiffInfo};

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
    pub known_advisories: Vec<CrateVersionRustSecAdvisory>,
}

#[derive(Debug, Clone)]
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
    pub insertions: u64,
    pub deletions: u64,
    pub modified_build_scripts: HashSet<String>, // Empty indicates no change in build scripts
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

#[derive(Debug, Clone)]
pub struct FileUnsafeChangeStats {
    pub file: String,
    pub change_type: Delta,
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
    cache: RefCell<HashMap<String, DepUpdateReviewReport>>,
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

        let dep_update_review_reports: Vec<DepUpdateReviewReport> = updated_deps
            .iter()
            .map(|dep| self.get_update_review(dep))
            .collect::<Result<_>>()?;

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
        if dep_change_info.old_version.is_none()
            || dep_change_info.new_version.is_none()
            || dep_change_info.new_version.as_ref().unwrap()
                <= dep_change_info.old_version.as_ref().unwrap()
        {
            return Err(anyhow!("dependency change does not represent an update"));
        }

        let name = &dep_change_info.name;
        if let Some(report) = self.get_update_review_report_from_cache(name) {
            return Ok(report);
        }

        let cratesio_analyzer = CratesioAnalyzer::new()?;
        let advisory_lookup = AdvisoryLookup::new()?;

        let old_version = dep_change_info.old_version.as_ref().unwrap().clone();
        let prior_version = VersionInfo {
            name: name.clone(),
            version: old_version.clone(),
            downloads: cratesio_analyzer.get_version_downloads(&name, &old_version)?,
            known_advisories: advisory_lookup
                .get_crate_version_advisories(&name, &old_version.to_string())?
                .iter()
                .filter(|advisory| advisory.metadata.withdrawn.is_none())
                .map(|advisory| Self::get_crate_version_rustsec_advisory(advisory))
                .collect(),
        };

        let new_version = dep_change_info.new_version.as_ref().unwrap().clone();
        let updated_version = VersionInfo {
            name: name.clone(),
            version: new_version.clone(),
            downloads: cratesio_analyzer.get_version_downloads(&name, &new_version)?,
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
        self.cache.borrow_mut().insert(name.clone(), report);
        self.get_update_review_report_from_cache(name)
            .ok_or_else(|| anyhow!("fatal cache error for update analyzer"))
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
        if let (name, Some(repository), Some(old_version), Some(new_version)) = (
            &dep_change_info.name,
            &dep_change_info.repository,
            &dep_change_info.old_version,
            &dep_change_info.new_version,
        ) {
            let diff_analyzer = DiffAnalyzer::new()?;
            let repo = diff_analyzer.get_git_repo(&name, &repository)?;
            let version_diff_info = match diff_analyzer.get_version_diff_info(
                &dep_change_info.name,
                &repo,
                &old_version,
                &new_version,
            ) {
                Ok(info) => info,
                Err(error) => match error.root_cause().downcast_ref::<HeadCommitNotFoundError>() {
                    Some(_err) => return Ok(None),
                    None => return Err(anyhow!("fatal error in fetching head commit")),
                },
            };

            let stats = version_diff_info.diff.stats()?;

            let modified_build_scripts: HashSet<String> = dep_change_info
                .build_script_paths
                .iter()
                .filter(|path| Self::is_file_modified(&path, &version_diff_info.diff))
                .map(|path| path.to_string())
                .collect();

            Ok(Some(VersionDiffStats {
                files_changed: stats.files_changed() as u64,
                insertions: stats.insertions() as u64,
                deletions: stats.deletions() as u64,
                modified_build_scripts,
            }))
        } else {
            // If repository, old version, or new version is none, there is no update diff
            Ok(None)
        }
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

            files_unsafe_change_stats.push(FileUnsafeChangeStats {
                file: diff_delta
                    .new_file()
                    .path()
                    .or_else(|| diff_delta.old_file().path())
                    .and_then(|path| path.to_str())
                    .ok_or_else(|| anyhow!("fatal error: diff contains no files"))?
                    .to_string(),
                change_type: diff_delta.status(),
                unsafe_delta: Self::get_unsafe_delta_from_rs_file_metrics(&new_file_unsafe_stats)
                    - Self::get_unsafe_delta_from_rs_file_metrics(&old_file_unsafe_stats),
                unsafe_status: new_file_unsafe_stats,
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

    fn get_update_review_report_from_cache(&self, key: &str) -> Option<DepUpdateReviewReport> {
        self.cache.borrow().get(key).cloned()
    }
}

#[cfg(test)]
mod test {
    use super::{
        DependencyType, DiffAnalyzer, PackageGraph, StandardFeatures, UpdateAnalyzer,
        VersionConflict::DirectTransitiveVersionConflict,
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
                assert_eq!(report.diff_stats.as_ref().unwrap().files_changed, 6);
                assert_eq!(report.diff_stats.as_ref().unwrap().insertions, 199);
                assert_eq!(report.diff_stats.as_ref().unwrap().deletions, 82);
                assert!(report
                    .diff_stats
                    .as_ref()
                    .unwrap()
                    .modified_build_scripts
                    .is_empty());
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
        assert_eq!(update_review_reports.dep_update_review_reports.len(), 2);
        for report in &update_review_reports.dep_update_review_reports {
            if report.name == "libc" {
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
                assert_eq!(report.diff_stats.as_ref().unwrap().files_changed, 121);
                assert_eq!(report.diff_stats.as_ref().unwrap().insertions, 19954);
                assert_eq!(report.diff_stats.as_ref().unwrap().deletions, 5032);
                assert_eq!(
                    report
                        .diff_stats
                        .as_ref()
                        .unwrap()
                        .modified_build_scripts
                        .len(),
                    1
                );
            }
        }
        println!("{:?}", update_review_reports);
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
    fn test_update_geiger() {
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
}
