//! This module abstracts the communication with GitHub API for a given crate

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, FixedOffset, Utc};
use guppy::graph::PackageMetadata;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitInfo {
    pub sha: String,
    pub commit: Commit,
    pub author: Option<User>,
    pub committer: Option<User>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Commit {
    pub author: Date,
    pub committer: Date,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Date {
    pub date: DateTime<FixedOffset>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    // can be null if the user is not registered on GitHub
    pub login: Option<String>,
}

pub struct GitHubReport {
    pub name: String,               // name of the crate
    pub repository: Option<String>, // repository url
    pub is_github_repo: bool,
    pub repo_stats: RepoStats,
    pub activity_metrics: ActivityMetrics,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RepoStats {
    pub full_name: Option<String>,
    pub default_branch: Option<String>,
    pub stargazers_count: u64,
    pub subscribers_count: u64,
    pub forks: u64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ActivityMetrics {
    pub days_since_last_commit: u64,
}

impl GitHubReport {
    fn new(name: String, repository: Option<String>) -> Self {
        //Returns a default GitHubReport with is_github_repo set as false
        GitHubReport {
            name,
            repository,
            is_github_repo: false,
            repo_stats: RepoStats {
                full_name: None,
                default_branch: None,
                ..Default::default()
            },
            activity_metrics: ActivityMetrics {
                ..Default::default()
            },
        }
    }
}

pub struct GitHubAnalyzer {
    client: reqwest::blocking::Client,
}

impl GitHubAnalyzer {
    fn construct_headers() -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("diem/whackadep"));

        let pat = std::env::var("GITHUB_TOKEN")?;
        let pat = format!("token {}", pat);
        let mut auth_value = HeaderValue::from_str(&pat)?;
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);

        Ok(headers)
    }

    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .default_headers(Self::construct_headers()?)
                .build()?,
        })
    }

    pub fn analyze_github(self, package: &PackageMetadata) -> Result<GitHubReport> {
        let name = package.name();
        let repository = match package.repository().and_then(|r| Url::from_str(r).ok()) {
            Some(repository) => repository,
            None => return Ok(GitHubReport::new(name.to_string(), None)),
        };

        let is_github_repo = Self::is_github_url(&repository);
        if !is_github_repo {
            return Ok(GitHubReport::new(
                name.to_string(),
                Some(repository.to_string()),
            ));
        }

        let repo_fullname = Self::get_github_repo_fullname(&repository)?;

        // Get Overall stats for a given repo
        let repo_stats = self.get_github_repo_stats(&repo_fullname)?;

        // Get the default branch
        let default_branch = repo_stats.default_branch.clone();
        let default_branch = match default_branch {
            Some(branch) => branch,
            None => return Err(anyhow!("No default branch found for package repository")),
        };

        // Get recent activity metrics
        let activity_metrics = self.get_activity_metrics(&repo_fullname, &default_branch)?;

        return Ok(GitHubReport {
            name: name.to_string(),
            repository: Some(repository.to_string()),
            is_github_repo,
            repo_stats,
            activity_metrics,
        });
    }

    fn is_github_url(url: &Url) -> bool {
        url.host_str()
            .map(|host| host == "github.com")
            .unwrap_or(false)
    }

    fn get_github_repo_fullname(repo_url: &Url) -> Result<String> {
        assert_eq!(
            Self::is_github_url(repo_url),
            true,
            "Repository is not from GitHub"
        );

        let mut segments = repo_url.path_segments().unwrap();
        let owner = segments
            .next()
            .ok_or_else(|| anyhow!("repository url missing owner"))?;
        let repo = segments
            .next()
            .map(|repo| repo.trim_end_matches(".git"))
            .ok_or_else(|| anyhow!("repository url missing repo"))?;
        return Ok(format!("{}/{}", owner, repo));
    }

    fn get_github_repo_stats(&self, repo_fullname: &String) -> Result<RepoStats> {
        let api_endpoint = format!("https://api.github.com/repos/{}", repo_fullname);
        let response = self.client.get(api_endpoint).send()?;

        if !response.status().is_success() {
            println!("repo_url: {}", repo_fullname);
            println!("{:?}", response.text());
            panic!("http request to GitHub failed");
        }

        Ok(response.json()?)
    }

    fn get_activity_metrics(
        self,
        repo_fullname: &String,
        default_branch: &String,
    ) -> Result<ActivityMetrics> {
        let days_since_last_commit = self
            .get_time_since_last_commit(&repo_fullname, &default_branch)?
            .num_days() as u64;

        Ok(ActivityMetrics {
            days_since_last_commit,
        })
    }

    fn get_time_since_last_commit(
        &self,
        repo_fullname: &String,
        default_branch: &String,
    ) -> Result<Duration> {
        let api_endpoint = format!(
            "https://api.github.com/repos/{}/commits?sha={}&per_page=1",
            repo_fullname, default_branch
        );
        let response = self.client.get(api_endpoint).send()?;

        if !response.status().is_success() {
            panic!("http request to GitHub failed, {:?}", response);
        }

        let response: Vec<CommitInfo> = response.json()?;
        // at lease one commit must be in the repository
        assert_eq!(response.is_empty(), false);

        let last_commit = &response[0];
        let last_commit_date = last_commit.commit.committer.date;

        let utc_now: DateTime<Utc> = Utc::now();
        let duration = utc_now.signed_duration_since(last_commit_date);
        assert!(duration.num_days() >= 0);
        Ok(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guppy::{graph::PackageGraph, MetadataCommand};
    use std::path::PathBuf;

    fn test_github_analyzer() -> GitHubAnalyzer {
        GitHubAnalyzer::new().unwrap()
    }

    fn get_test_graph() -> PackageGraph {
        MetadataCommand::new()
            .current_dir(PathBuf::from("resources/test/valid_dep"))
            .build_graph()
            .unwrap()
    }

    fn get_test_repo(package_name: &str) -> (String, String) {
        let graph = get_test_graph();
        let pkg = graph.packages().find(|p| p.name() == package_name).unwrap();

        let repository = pkg.repository().unwrap();
        let url = Url::from_str(repository).unwrap();
        let fullname = GitHubAnalyzer::get_github_repo_fullname(&url).unwrap();

        let github_analyzer = test_github_analyzer();
        let report = github_analyzer.analyze_github(&pkg).unwrap();
        let default_branch = report.repo_stats.default_branch.unwrap();

        (fullname, default_branch)
    }

    #[test]
    fn test_github_stats_for_libc() {
        let github_analyzer = test_github_analyzer();

        let graph = get_test_graph();
        let pkg = graph.packages().find(|p| p.name() == "libc").unwrap();
        let report = github_analyzer.analyze_github(&pkg).unwrap();

        assert_eq!(report.is_github_repo, true);

        // Relying on Libc to have at least one star on GitHub
        assert!(report.repo_stats.stargazers_count > 0);
    }

    #[test]
    fn test_github_stats_for_gitlab() {
        let github_analyzer = test_github_analyzer();

        let graph = get_test_graph();
        let pkg = graph.packages().find(|p| p.name() == "gitlab").unwrap();
        let report = github_analyzer.analyze_github(&pkg).unwrap();

        assert_eq!(report.is_github_repo, false);
        assert_eq!(report.repo_stats.stargazers_count, 0);
    }

    #[test]
    fn test_github_time_since_last_commit() {
        let github_analyzer = test_github_analyzer();
        let (fullname, default_branch) = get_test_repo("octocrab");

        let time_since_last_commit = github_analyzer
            .get_time_since_last_commit(&fullname, &default_branch)
            .unwrap();
        assert_eq!(time_since_last_commit.num_nanoseconds().unwrap() > 0, true)
    }
}
