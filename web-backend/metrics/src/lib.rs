use anyhow::Result;
use mongodb::bson;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod db;
mod external;
mod git;
pub mod rust;

use db::Db;
use git::Repo;
use rust::RustAnalysis;

// The flow:
// 1. initialize repo if not done
// 2. git pull to get latest change
// 3. run metrics to "extract" information about our dependencies
//    this step only works for rust stuff atm
// 4. check for updates
// 5. load it in DB

pub struct Metrics {
    repo: Repo,
    db: Db,
}

impl Metrics {
    pub async fn new(repo_url: &str, repo_path: &Path) -> Result<Self> {
        // 1. initialize repo if not done
        println!("getting diem/diem repo");
        let repo = match Repo::new(repo_path) {
            Ok(repo) => repo,
            Err(_) => {
                println!("didn't have it, cloning it for the first time");
                Repo::clone(repo_url, repo_path)?
            }
        };

        // 2. create client to database
        let db = Db::new().await?;

        //
        Ok(Self { repo, db })
    }

    // function use to start an analyze
    pub async fn start_analysis(&self) -> Result<()> {
        // 1. pull latest changes on the repo
        println!("pulling latest changes");
        self.repo.update()?;

        // 2. get metadata
        let commit = self.repo.head().expect("couldn't get HEAD hash");
        println!("current commit: {}", commit);

        // 3. if we have already checked that commit, we can skip retrieving dependency info from Cargo.lock
        let rust_analysis = match self.db.find(&commit).await? {
            None => {
                // 4. retrieve dependencies
                RustAnalysis::get_dependencies(&self.repo.repo_folder)?
            }
            Some(document) => bson::from_document(document).map_err(anyhow::Error::msg)?,
        };

        // 5. analyze dependencies

        // TKTK...
        // note that dependencies might already have been analyzed here...
        // so what to do?
        // not do anything if nothing has changed
        // detect changes?
        // update in place?

        // 6. store analysis in db
        println!("analysis done, storing in db...");
        let analysis = Analysis {
            commit: commit,
            rust_dependencies: rust_analysis,
        };

        let analysis = bson::to_bson(&analysis).unwrap();
        let document = analysis.as_document().unwrap();
        self.db.write(document.to_owned()).await;

        //
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Analysis {
    commit: String,
    rust_dependencies: RustAnalysis,
}
