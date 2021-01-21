use anyhow::Result;
use std::path::Path;

mod db;
mod git;
mod external;

use db::Db;
use git::Repo;

const REPO_URL: &str = "https://github.com/diem/diem.git";

// 1. initialize repo if not done
// 2. git pull to get latest change
// 3. run metrics to "extract"
// 4. transform information
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

    pub fn update(&self) -> Result<()> {
        self.repo.update()?;
        Ok(())
    }

    pub fn get_dependencies() {
        cargo guppy select --kind ThirdParty > ../third_party.deps
        cargo guppy select --kind DirectThirdParty > ../direct_third_party.deps
    }

    pub fn store_analysis(&self) {
        // 3. get metadata
        let commit = self.repo.head().expect("couldn't get HEAD hash");
    }
}
