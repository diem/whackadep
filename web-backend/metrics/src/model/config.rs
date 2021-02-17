//! This module abstracts the database (mongodb)
//! by providing functions to read and write specific documents.

use super::Db;
use anyhow::{anyhow, Result};
use mongodb::bson::{self, doc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Repo {
    pub repo: String,
    pub trusted_crates: Vec<String>,
    pub snoozed_crates: Vec<String>,
}

pub struct Config(Db);

impl Config {
    const COLLECTION: &'static str = "config";

    /// to use the config database, call `Config::new(db.clone())`
    /// on a [`crate::model::Db`].
    pub fn new(db: Db) -> Self {
        Self(db)
    }

    /// add a new repository configuration
    pub async fn add_new_repo(&self, repo: &str) -> Result<()> {
        // check if the repo already exists
        let filter = doc! {
            repo: repo.to_string(),
        };
        let found = self
            .0
            .find_one(Self::COLLECTION, Some(filter), None)
            .await?
            .is_some();
        if found {
            return Err(anyhow!("repo already exists"));
        }

        // if not, create it
        let repo = Repo {
            repo: repo.to_string(),
            trusted_crates: Vec::new(),
            snoozed_crates: Vec::new(),
        };
        let repo = bson::to_bson(&repo).unwrap();
        let document = repo.as_document().unwrap();
        self.0.write(Self::COLLECTION, document.to_owned()).await
    }

    /// remove a repository configuration
    pub async fn remove_repo(&self, repo: &str) -> Result<()> {
        self.0
            .delete_one(
                Self::COLLECTION,
                doc! {
                  repo: repo,
                },
                None,
            )
            .await
    }

    /// obtain the saved repositories configuration
    pub async fn get_repos(&self) -> Result<Vec<Repo>> {
        let documents = self.0.find(Self::COLLECTION, None, None).await?;
        let documents = documents
            .into_iter()
            .map(|doc| bson::from_document(doc).map_err(anyhow::Error::msg));
        // TODO: log the Err
        Ok(documents.filter_map(Result::ok).collect())
    }
}
