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
        };

        Ok(dependency_report)
    }
}
