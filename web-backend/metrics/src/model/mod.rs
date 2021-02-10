use anyhow::Result;
use mongodb::{
    bson::Document,
    options::{ClientOptions, FindOneOptions},
    Client, Database,
};
use std::env;
use tracing::info;

mod config;
mod dependencies;

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

    pub async fn write(&self, document: Document) -> Result<()> {
        let insert_result = self
            .0
            .collection("dependencies")
            .insert_one(document, None)
            .await
            .map_err(anyhow::Error::msg)?;
        info!("New document ID: {}", insert_result.inserted_id);
        Ok(())
    }

    pub async fn find_one(
        &self,
        collection: &str,
        filter: Document,
        options: Option<FindOneOptions>,
    ) -> Result<Option<Document>> {
        self.0
            .collection(collection)
            .find_one(filter, options)
            .await
            .map_err(anyhow::Error::msg)
    }
}
