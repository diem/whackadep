use anyhow::{Context, Result};
use mongodb::bson;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::db::Db;
use crate::git::Repo;
use crate::rust::RustAnalysis;

// The flow:
// 1. initialize repo if not done
// 2. git pull to get latest change
// 3. run metrics to "extract" information about our dependencies
//    this step only works for rust stuff atm
// 4. check for updates
// 5. load it in DB

#[derive(Serialize, Deserialize)]
pub struct Analysis {
    commit: String,
    rust_dependencies: RustAnalysis,
}

impl Analysis {
    pub async fn analyze(repo_url: &str, repo_path: &Path) -> Result<()> {
        // 1. initialize repo if not done
        println!("getting diem/diem repo");
        let repo = match Repo::new(repo_path) {
            Ok(repo) => repo,
            Err(_) => {
                println!("didn't have it, cloning it for the first time");
                Repo::clone(repo_url, repo_path)?
            }
        };

        // 3. pull latest changes on the repo
        println!("pulling latest changes");
        repo.update()?;

        // 4. get metadata
        let commit = repo.head().expect("couldn't get HEAD hash");
        println!("current commit: {}", commit);

        // 5. if we have already checked that commit, we can skip retrieving dependency info from Cargo.lock
        let rust_analysis = match Db::find(&commit)? {
            None => {
                // 4. retrieve dependencies
                RustAnalysis::get_dependencies(&repo.repo_folder).await?
            }
            Some(document) => {
                println!("commit already present in db");
                let analysis: Analysis = bson::from_document(document)
                    .map_err(anyhow::Error::msg)
                    .context("Failed to deserialize analysis from database")?;
                analysis.rust_dependencies
            }
        };

        // 6. analyze dependencies

        // TKTK...
        // note that dependencies might already have been analyzed here...
        // so what to do?
        // not do anything if nothing has changed
        // detect changes?
        // update in place?

        // 7. store analysis in db
        println!("analysis done, storing in db...");
        let analysis = Analysis {
            commit: commit,
            rust_dependencies: rust_analysis,
        };

        let analysis = bson::to_bson(&analysis).unwrap();
        let document = analysis.as_document().unwrap();
        Db::write(document.to_owned())
    }
}
