use crate::analysis::Analysis;
use anyhow::{anyhow, Result};
use mongodb::{
    bson::{self, doc, Document},
    options::{ClientOptions, FindOneOptions},
    Client,
};
use old_tokio::runtime::Runtime as OldRuntime;
use std::env;

// TODO: this is not great! We spin a new runtime for every request. instead create a structure that is initialized once with a runtime, and re-use it over and over. At the same time, we're not doing db queries like crazy so, who cares?
pub struct Db;

impl Db {
    /// this should be called by every query, as different queries should create new connections to the db
    /// (since different queries might concurrently query the database).
    async fn new() -> Result<mongodb::Database> {
        let mongodb_uri =
            env::var("MONGODB_URI").unwrap_or("mongodb://root:password@mongo:27017".to_string());
        println!("using following mongodb uri: {}", mongodb_uri);

        // parse a connection string into an options struct
        let mut client_options = ClientOptions::parse(&mongodb_uri).await?;
        client_options.app_name = Some("Metrics".to_string());

        // get a handle to the deployment
        let client = Client::with_options(client_options)?;

        //
        println!("databases:");
        for name in client.list_database_names(None, None).await? {
            println!("- {}", name);
        }

        // get a handle to whackadep database
        let db = client.database("whackadep");

        //
        Ok(db)
    }

    pub fn write(document: Document) -> Result<()> {
        let mut rt = OldRuntime::new().unwrap();
        rt.block_on(async {
            let db = Self::new().await.map_err(anyhow::Error::msg)?;
            let insert_result = db
                .collection("dependencies")
                .insert_one(document, None)
                .await
                .map_err(anyhow::Error::msg)?;
            println!("New document ID: {}", insert_result.inserted_id);
            Ok(())
        })
    }

    pub fn find(commit: &str) -> Result<Option<Document>> {
        let mut rt = OldRuntime::new().unwrap();
        rt.block_on(async {
            let db = Self::new().await.map_err(anyhow::Error::msg)?;
            db.collection("dependencies")
                .find_one(
                    doc! {
                          "commit": commit,
                    },
                    None,
                )
                .await
                .map_err(anyhow::Error::msg)
        })
    }

    pub fn get_dependencies() -> Result<Analysis> {
        let find_options = FindOneOptions::builder()
            .sort(doc! {
                "_id": -1
            })
            .build();

        let mut rt = OldRuntime::new().unwrap();
        let dependencies: Result<bson::Document> = rt.block_on(async {
            let db = Self::new().await.map_err(anyhow::Error::msg)?;
            db.collection("dependencies")
                .find_one(None, find_options)
                .await
                .map_err(anyhow::Error::msg)?
                .ok_or(anyhow!("could not find any dependencies"))
        });

        // deserialize
        bson::from_document(dependencies?).map_err(anyhow::Error::msg)
    }

    // config should return:
    // {
    // trusted_dependencies: HashMap<name, reasons>,
    // paused_dependencies: ...
    // }
    pub fn get_config() -> Result<Document> {
        unimplemented!();
    }
}
