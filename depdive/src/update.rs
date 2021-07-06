//! This module abstracts analyses for dependency update review.

use crate::cratesio::CratesioAnalyzer;
use anyhow::{anyhow, Result};
use guppy::graph::{
    cargo::{CargoOptions, CargoResolverVersion},
    feature::{FeatureFilter, StandardFeatures},
    summaries::{
        diff::{SummaryDiff, SummaryDiffStatus},
        Summary, SummaryId,
    },
    PackageGraph,
};
use semver::Version;

use crate::diff::{DiffAnalyzer, HeadCommitNotFoundError};

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
}

#[derive(Debug)]
pub struct UpdateReviewReport {
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
}

pub struct VersionChangeInfo {
    pub old_version: Option<Version>, // None when a dep is added
    pub new_version: Option<Version>, // None when a dep is removed
}

#[derive(Debug)]
pub struct VersionDiffStats {
    pub files_changed: u64,
    pub insertions: u64,
    pub deletions: u64,
}

pub struct UpdateAnalyzer;

impl UpdateAnalyzer {
    pub fn analyze_updates(
        prior_graph: &PackageGraph,
        post_graph: &PackageGraph,
    ) -> Result<Vec<UpdateReviewReport>> {
        // Analyzing with default options
        Self::analyze_updates_with_options(
            prior_graph,
            post_graph,
            &Self::get_default_cargo_options(),
            StandardFeatures::All,
        )
    }

    pub fn analyze_updates_with_options<'a>(
        prior_graph: &'a PackageGraph,
        post_graph: &'a PackageGraph,
        cargo_opts: &CargoOptions,
        feature_filter: impl FeatureFilter<'a>,
    ) -> Result<Vec<UpdateReviewReport>> {
        // Get the changed dependency stats
        let dep_change_infos =
            Self::compare_pacakge_graphs(&prior_graph, &post_graph, cargo_opts, feature_filter)?;

        // Filter version updates
        let updated_deps: Vec<DependencyChangeInfo> = dep_change_infos
            .iter()
            .filter(|dep| {
                !dep.old_version.is_none()
                    && !dep.new_version.is_none()
                    && dep.new_version.as_ref().unwrap() > dep.old_version.as_ref().unwrap()
            })
            .cloned()
            .collect();
        // TODO: add reporting for version downgrades, add, and remove

        let update_review_reports: Vec<UpdateReviewReport> = updated_deps
            .iter()
            .map(|dep| Self::get_update_review(dep))
            .collect::<Result<_>>()?;

        Ok(update_review_reports)
    }

    fn get_default_cargo_options() -> CargoOptions<'static> {
        let mut cargo_opts = CargoOptions::new();
        cargo_opts.set_version(CargoResolverVersion::V2);
        cargo_opts.set_include_dev(true);
        cargo_opts
    }

    pub fn compare_pacakge_graphs<'a>(
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

    pub fn get_summary<'a>(
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

    pub fn get_dependency_change_info(
        prior_graph: &PackageGraph,
        post_graph: &PackageGraph,
        summary_id: &SummaryId,
        summary_diff_status: &SummaryDiffStatus,
        dep_type: DependencyType,
    ) -> Result<DependencyChangeInfo> {
        let name = summary_id.name.clone();
        let version_change_info =
            Self::get_version_change_info_from_summarydiff(&summary_id, &summary_diff_status);
        let repository = Self::get_repository_from_graphs(&vec![prior_graph, post_graph], &name);

        let old_version = version_change_info.old_version;
        let new_version = version_change_info.new_version;

        Ok(DependencyChangeInfo {
            name,
            repository,
            dep_type,
            old_version,
            new_version,
        })
    }

    pub fn get_version_change_info_from_summarydiff(
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
                if !version.is_none() {
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

    pub fn get_repository_from_graphs(
        graphs: &Vec<&PackageGraph>,
        crate_name: &str,
    ) -> Option<String> {
        for graph in graphs {
            let repository = Self::get_repository_from_graph(graph, crate_name);
            if !repository.is_none() {
                return repository;
            }
        }
        None
    }

    pub fn get_repository_from_graph(graph: &PackageGraph, crate_name: &str) -> Option<String> {
        let package = graph.packages().find(|p| p.name() == crate_name)?;
        let repository = package.repository()?.to_string();
        Some(repository)
    }

    pub fn get_update_review(dep_change_info: &DependencyChangeInfo) -> Result<UpdateReviewReport> {
        if dep_change_info.old_version.is_none()
            || dep_change_info.new_version.is_none()
            || !(dep_change_info.new_version.as_ref().unwrap()
                > dep_change_info.old_version.as_ref().unwrap())
        {
            return Err(anyhow!("dependency change does not represent an update"));
        }

        let name = &dep_change_info.name;
        let cratesio_analyzer = CratesioAnalyzer::new()?;

        let old_version = dep_change_info.old_version.as_ref().unwrap().clone();
        let prior_version = VersionInfo {
            name: name.clone(),
            version: old_version.clone(),
            downloads: cratesio_analyzer.get_version_downloads(&name, &old_version)?,
        };

        let new_version = dep_change_info.new_version.as_ref().unwrap().clone();
        let updated_version = VersionInfo {
            name: name.clone(),
            version: old_version.clone(),
            downloads: cratesio_analyzer.get_version_downloads(&name, &new_version)?,
        };

        let diff_stats = Self::analyze_version_diff(&dep_change_info)?;

        Ok(UpdateReviewReport {
            name: dep_change_info.name.clone(),
            prior_version,
            updated_version,
            diff_stats,
        })
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
            let diff = match diff_analyzer.get_version_diff(
                &dep_change_info.name,
                &repo,
                &old_version,
                &new_version,
            ) {
                Ok(diff) => diff,
                Err(error) => match error.root_cause().downcast_ref::<HeadCommitNotFoundError>() {
                    Some(_err) => return Ok(None),
                    None => return Err(anyhow!("fatal error in fetching head commit")),
                },
            };

            let stats = diff.stats()?;

            Ok(Some(VersionDiffStats {
                files_changed: stats.files_changed() as u64,
                insertions: stats.insertions() as u64,
                deletions: stats.deletions() as u64,
            }))
        } else {
            // If repository, old version, or new version is none, there is no update diff
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use super::{DependencyType, PackageGraph, StandardFeatures, UpdateAnalyzer};
    use crate::diff::trim_remote_url;
    use guppy::CargoMetadata;

    fn get_test_graph_pairs() -> (PackageGraph, PackageGraph) {
        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/prior_dep_change_metadata.json"
        ))
        .unwrap();
        let prior = metadata.build_graph().unwrap();

        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/post_dep_change_metadata.json"
        ))
        .unwrap();
        let post = metadata.build_graph().unwrap();

        (prior, post)
    }

    #[test]
    fn test_update_compare_package_graph() {
        let pair = get_test_graph_pairs();
        let prior = pair.0;
        let post = pair.1;

        let dep_change_infos = UpdateAnalyzer::compare_pacakge_graphs(
            &prior,
            &post,
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
                .filter(|dep| dep.old_version.is_none() && !dep.new_version.is_none())
                .count()
        );

        // Deps removed
        assert_eq!(
            10,
            dep_change_infos
                .iter()
                .filter(|dep| !dep.old_version.is_none() && dep.new_version.is_none())
                .count()
        );

        // Deps version changed
        assert_eq!(
            2,
            dep_change_infos
                .iter()
                .filter(|dep| !dep.old_version.is_none() && !dep.new_version.is_none())
                .count()
        );
    }

    #[test]
    fn get_repository_from_graphs() {
        let pair = get_test_graph_pairs();
        let graphs = vec![&pair.0, &pair.1];

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
    fn test_update_review_report() {
        let pair = get_test_graph_pairs();
        let prior = pair.0;
        let post = pair.1;

        let update_review_reports = UpdateAnalyzer::analyze_updates(&prior, &post).unwrap();
        assert_eq!(update_review_reports.len(), 2);
        for report in &update_review_reports {
            if report.name == "guppy" {
                assert_eq!(report.diff_stats.as_ref().unwrap().files_changed, 26);
                assert_eq!(report.diff_stats.as_ref().unwrap().insertions, 373);
                assert_eq!(report.diff_stats.as_ref().unwrap().deletions, 335);
            }
        }
        println!("{:?}", update_review_reports);
    }
}
