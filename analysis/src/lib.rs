//! This crate contains a number of analyses that can be run on dependencies

use anyhow::Result;
use guppy::graph::PackageMetadata;
use tabled::Tabled;

mod cratesio;
mod github;

#[derive(Tabled)]
pub struct DependencyReport<'a> {
    pub name: &'a str,
    pub has_build_script: bool,
    pub hosted_on_cratesio: bool,
    pub cratesio_downloads: u64,
    pub cratesio_dependents: u64,
    pub hosted_on_github: bool,
    pub github_stars: u64,
    pub github_subscribers: u64,
    pub github_forks: u64,
    pub open_issues: u64,
    pub days_since_last_commit_on_default_branch: u64,
    pub days_since_last_open_issue: u64,
    pub open_issues_labeld_bug: u64,
    pub open_issues_labeled_security: u64,
    pub past_days_for_recent_stats: u64,
    pub recent_commits: u64,
    pub recent_committers: u64,
}

pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    pub fn analyze_dep<'a>(&self, package: &PackageMetadata<'a>) -> Result<DependencyReport<'a>> {
        let name = package.name();
        let has_build_script = package.has_build_script();

        // Crates.io analysis
        let cratesio_report = cratesio::CratesioAnalyzer::new()?;
        let cratesio_report = cratesio_report.analyze_cratesio(package)?;

        // GitHub API analysis
        let github_report = github::GitHubAnalyzer::new()?;
        let github_report = github_report.analyze_github(package)?;

        let dependency_report = DependencyReport {
            name,
            has_build_script,
            hosted_on_cratesio: cratesio_report.is_hosted,
            cratesio_downloads: cratesio_report.downloads,
            cratesio_dependents: cratesio_report.dependents,
            hosted_on_github: github_report.is_github_repo,
            github_stars: github_report.repo_stats.stargazers_count,
            github_subscribers: github_report.repo_stats.subscribers_count,
            github_forks: github_report.repo_stats.forks,
            open_issues: github_report.repo_stats.open_issues,
            days_since_last_commit_on_default_branch: github_report
                .activity_metrics
                .days_since_last_commit,
            days_since_last_open_issue: github_report
                .activity_metrics
                .days_since_last_open_issue
                .unwrap_or(0),
            open_issues_labeld_bug: github_report.activity_metrics.open_issues_labeld_bug,
            open_issues_labeled_security: github_report
                .activity_metrics
                .open_issues_labeled_security,
            past_days_for_recent_stats: github_report.activity_metrics.recent_activity.past_days,
            recent_commits: github_report.activity_metrics.recent_activity.commits,
            recent_committers: github_report.activity_metrics.recent_activity.committers,
        };

        Ok(dependency_report)
    }
}
