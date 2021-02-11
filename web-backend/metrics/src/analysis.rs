//! This module implements the code to analyze a repository's dependencies.

use anyhow::Result;
use chrono::prelude::*;
use crypto::{digest::Digest, md5::Md5};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{error, info};

use crate::git::Repo;
use crate::model::{Db, Dependencies};
use crate::rust::RustAnalysis;

//
// Data that is stored in MongoDB
//

#[derive(Serialize, Deserialize, Debug)]
/// An analysis result. It contains the commit that was analyzed, as well as the results of the analysis on dependencies.
/// At the moment it only contains analysis results for Rust dependencies.
pub struct Analysis {
    // The full repository link (e.g. https://github.com/diem/diem.git)
    repository: String,
    /// The SHA-1 hash indicating the exact commit used to analyze the given repository.
    commit: String,
    // the time at which the analysis was done
    timestamp: DateTime<Utc>,
    // previous analysis
    previous_analysis: Option<PreviousAnalysis>,

    // per-languages results
    /// The result of the rust dependencies analysis
    rust_dependencies: RustAnalysis,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PreviousAnalysis {
    commit: String,
    timestamp: DateTime<Utc>,
}

//
// App
//

pub struct MetricsApp {
    db: Db,
}

impl MetricsApp {
    pub async fn new() -> Result<Self> {
        let db = Db::new(None, None, None, None).await?;
        Ok(Self { db: db })
    }

    /// The analyze function does the following:
    /// 1. It initializes a given repository (if not already done previously).
    /// 2. It pulls the latest changes.
    /// 3. It records the commit pointed by the HEAD of the repository.
    /// 4. It runs language-dependent analysis to "extract" information about our dependencies (this step only works for Rust dependencies stuff at the moment).
    /// 5. It stores the results in the database.
    pub async fn refresh(&self, repo_url: &str, repo_dir: &Path) -> Result<()> {
        // 1. initialize repo if not done
        let mut md5 = Md5::new();
        md5.input_str(repo_url);
        let repo_path = repo_dir.join(&md5.result_str());
        info!("getting diem/diem repo");
        let repo = match Repo::new(&repo_path) {
            Ok(repo) => repo,
            Err(_) => {
                info!("cloning {} into {}", repo_url, repo_path.to_string_lossy());
                Repo::clone(repo_url, &repo_path).await?
            }
        };

        // 2. pull latest changes on the repo
        info!("pulling latest changes");
        repo.update().await?;

        // 3. get metadata
        let commit = repo.head().await.expect("couldn't get HEAD hash");
        info!("current commit: {}", commit);

        // 4. get previous analysis
        let db = Dependencies::new(self.db.clone());
        let previous_analysis = match db.get_last_analysis(repo_url).await {
            Ok(maybe_prev) => maybe_prev,
            Err(e) => {
                error!(
                    "couldn't get previous analysis, perhaps the format changed: {}",
                    e
                );
                None
            }
        };

        // 5. run analysis for different languages
        // (at the moment we only have Rust)
        let previous_rust_analysis = previous_analysis.as_ref().map(|x| &x.rust_dependencies);
        let is_diem = (repo_url == "https://github.com/diem/diem.git");
        let rust_analysis =
            RustAnalysis::get_dependencies(&repo.repo_folder, previous_rust_analysis, is_diem)
                .await?;

        // 6. store analysis in db
        info!("analysis done, storing in db...");

        // 4. get previous analysis
        let previous_analysis = if let Some(previous_analysis) = &previous_analysis {
            Some(PreviousAnalysis {
                commit: previous_analysis.commit.clone(),
                timestamp: previous_analysis.timestamp.clone(),
            })
        } else {
            None
        };
        let analysis = Analysis {
            commit: commit,
            repository: repo_url.to_string(),
            timestamp: Utc::now(),
            previous_analysis: previous_analysis,
            rust_dependencies: rust_analysis,
        };
        db.write_analysis(analysis).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_analysis() {
        let temp_dir = tempdir().unwrap();
        MetricsApp::new()
            .await
            .unwrap()
            .refresh("https://github.com/diem/diem.git", temp_dir.path())
            .await
            .unwrap();
    }
}
