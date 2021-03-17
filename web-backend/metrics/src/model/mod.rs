use anyhow::{anyhow, Result};
use mongodb::{
    bson::Document,
    options::{ClientOptions, DeleteOptions, FindOneOptions, FindOptions},
    Client, Database,
};
use std::env;
use tracing::info;

mod config;
mod dependencies;

pub use config::Config;
pub use dependencies::Dependencies;

#[derive(Clone)]
pub struct Db(Database);

impl Db {
    /// this should be called by every query, as different queries should create new connections to the db
    /// (since different queries might concurrently query the database).
    pub async fn new(
        host: Option<&str>,
        port: Option<&str>,
        user: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self> {
        // get MongoDB parameters
        let host = host.unwrap_or("mongo");
        let port = port.unwrap_or("27017");

        let user = user.unwrap_or("root");
        let password = password.unwrap_or("password");

        let mongodb_uri = env::var("MONGODB_URI")
            .ok()
            .unwrap_or_else(|| format!("mongodb://{}:{}@{}:{}", user, password, host, port));

        info!("using following mongodb uri: {}", mongodb_uri);

        // create a MongoDB client to the whackadep database
        let mut client_options = ClientOptions::parse(&mongodb_uri).await?;
        client_options.app_name = Some("Metrics".to_string());
        let client = Client::with_options(client_options)?;
        let db = client.database("whackadep");

        //
        Ok(Db(db))
    }

    pub async fn write(&self, collection: &str, document: Document) -> Result<()> {
        let insert_result = self
            .0
            .collection(collection)
            .insert_one(document, None)
            .await
            .map_err(anyhow::Error::msg)?;
        info!("New document ID: {}", insert_result.inserted_id);
        Ok(())
    }

    pub async fn find_one(
        &self,
        collection: &str,
        filter: Option<Document>,
        options: Option<FindOneOptions>,
    ) -> Result<Option<Document>> {
        self.0
            .collection(collection)
            .find_one(filter, options)
            .await
            .map_err(anyhow::Error::msg)
    }

    pub async fn find(
        &self,
        collection: &str,
        filter: Option<Document>,
        options: Option<FindOptions>,
    ) -> Result<Vec<Document>> {
        use futures::StreamExt;
        let cursor = self
            .0
            .collection(collection)
            .find(filter, options)
            .await
            .map_err(anyhow::Error::msg)?;
        let res: Vec<mongodb::error::Result<Document>> = cursor.collect().await;
        // TODO: log the Err
        let res: Vec<Document> = res.into_iter().filter_map(Result::ok).collect();
        Ok(res)
    }

    pub async fn delete_one(
        &self,
        collection: &str,
        filter: Document,
        options: Option<DeleteOptions>,
    ) -> Result<()> {
        let res = self
            .0
            .collection(collection)
            .delete_one(filter, options)
            .await
            .map_err(anyhow::Error::msg)?;
        if res.deleted_count != 1 {
            return Err(anyhow!(
                "deleted inconsistent number of repo config: {}",
                res.deleted_count
            ));
        }
        Ok(())
    }
}
