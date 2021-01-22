use anyhow::Result;
use mongodb::bson::doc;
use mongodb::bson::Document;
use mongodb::{options::ClientOptions, Client};
use std::env;

// 1. initialize DB if not
// 2. create index on date

pub struct Db(mongodb::Database);

impl Db {
    pub async fn new() -> Result<Self> {
        let mongodb_uri = env::var("MONGODB_URI").unwrap_or("mongodb://mongo:27017".to_string());
        println!("using following mongodb uri: {}", mongodb_uri);

        // parse a connection string into an options struct
        let mut client_options = ClientOptions::parse(&mongodb_uri).await?;
        client_options.app_name = Some("Metrics".to_string());

        // get a handle to the deployment
        let client = Client::with_options(client_options)?;

        // ping to check connection
        client
            .database("whackadep")
            .run_command(doc! {"ping": 1}, None)
            .await?;
        println!("Connected successfully.");

        //
        println!("databases:");
        for name in client.list_database_names(None, None).await? {
            println!("- {}", name);
        }

        // get a handle to whackadep database
        let db = client.database("whackadep");

        //
        Ok(Self(db))
    }

    pub async fn write(&self, document: Document) {
        let insert_result = self
            .0
            .collection("dependencies")
            .insert_one(document, None)
            .await
            .unwrap();
        println!("New document ID: {}", insert_result.inserted_id);
    }

    pub async fn find(&self, commit: &str) -> Result<Option<Document>> {
        self.0
            .collection("dependencies")
            .find_one(
                doc! {
                      "commit": commit,
                },
                None,
            )
            .await
            .map_err(anyhow::Error::msg)
    }
}
