//! This module abstracts the communication with GitHub API for a given crate

use anyhow::{anyhow, Result};
use guppy::graph::PackageMetadata;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RepoStats {
    pub full_name: Option<String>,
    pub stargazers_count: u64,
    pub subscribers_count: u64,
    pub forks: u64,
}

pub struct GitHubReport {
    pub name: String,               // name of the crate
    pub repository: Option<String>, // repository url
    pub is_github_repo: bool,
    pub repo_stats: RepoStats,
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

        return Ok(GitHubReport {
            name: name.to_string(),
            repository: Some(repository.to_string()),
            is_github_repo,
            repo_stats,
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
}
