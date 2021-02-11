//! This module abstracts the database (mongodb)
//! by providing functions to read and write specific documents.

use super::Db;
use crate::analysis::Analysis;
use anyhow::Result;
use mongodb::{
    bson::{self, doc, Document},
    options::FindOneOptions,
};

pub struct Dependencies(Db);

impl Dependencies {
    const COLLECTION: &'static str = "dependencies";

    pub fn new(db: Db) -> Self {
        Self(db)
    }

    /// write an analysis to storage
    pub async fn write_analysis(&self, analysis: Analysis) -> Result<()> {
        let analysis = bson::to_bson(&analysis).unwrap();
        let document = analysis.as_document().unwrap();
        self.0.write(Self::COLLECTION, document.to_owned()).await
    }

    /// find an analysis by repo and commit
    pub async fn find_commit(&self, repo: &str, commit: &str) -> Result<Option<Document>> {
        let filter = doc! {
            "repository": repo,
            "commit": commit,
        };
        self.0.find_one(Self::COLLECTION, Some(filter), None).await
    }

    /// get the last analysis for a specific repo
    pub async fn get_last_analysis(&self, repo: &str) -> Result<Option<Analysis>> {
        let filter = doc! {
            "repository": repo,
        };
        let find_options = FindOneOptions::builder()
            .sort(doc! {
                "_id": -1,
            })
            .build();

        let analysis = self
            .0
            .find_one(Self::COLLECTION, Some(filter), Some(find_options))
            .await;

        // did we find anything?
        let analysis = match analysis? {
            Some(x) => x,
            None => return Ok(None),
        };

        // deserialize
        bson::from_document(analysis)
            .map(|analysis| Some(analysis))
            .map_err(anyhow::Error::msg)
    }
}
