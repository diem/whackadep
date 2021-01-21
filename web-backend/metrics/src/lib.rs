use anyhow::Result;
use std::path::Path;

pub mod db;
mod git;
mod external;
mod languages;

use db::Db;
use git::Repo;

const REPO_URL: &str = "https://github.com/diem/diem.git";

// The flow:
// 1. initialize repo if not done
// 2. git pull to get latest change
// 3. run metrics to "extract" information about our dependencies
//    this step only works for rust stuff atm
// 4. check for updates 
// 5. load it in DB

struct Metrics {
    repo: Repo,
    db: Db,
}

impl Metrics {
    pub async fn new(repo_url: &str, repo_path: &Path) -> Result<Self> {
        // 1. initialize repo if not done
        let repo = match Repo::new(repo_path) {
            Ok(x) => x,
            Err(_) => Repo::clone(repo_url, repo_path)?,
        };

        // 2. pull to get latest changes
        repo.update()?;

        // 3. create client to database
        let db = Db::new().await?;

        //
        Ok(Self { repo, db })
    }

    // function use to start an analyze
    pub fn start_analysis(&self) -> Result<()> {
        // 1. update 
        self.repo.update()?;

        // 2. get metadata
        let commit = self.repo.head().expect("couldn't get HEAD hash");

        // 3. if we have already checked that commit, we can skip retrieving dependency info from Cargo.lock

        // 4. retrieve dependencies

        // 5. analyze dependencies

        // 6. store analysis in db

        //
        Ok(())
    }
}

impl Metrics {
    fn store_analysis(&self) {
    }
}

// represent a run of the analysis
struct Analysis {
    commit: String,

    // not including dev dependencies
    directDependencies: Vec<Dependency>,
    indirectDependencies: Vec<Dependency>,

    // dev dependencies
    devDependencies: Vec<Dependency>,

    // updates available
    availableUpdates: Vec<DependencyChange>,
}

enum Language {
    Rust,
    Dockerfile,
    Npm,
}

enum Source {
    CratesIo,
    Github,
}

struct Dependency {
    name: String,
    language: Language,
    repo: Source,
    version: String,
}

struct DependencyChange {
    name: String,
    language: Language,
    repo: Source,
    version: String,
// name of committers ?
// what info do we want to carry in a dependency change?
}
