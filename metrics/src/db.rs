use anyhow::Result;
use mongodb::{options::ClientOptions, Client};
use mongodb::bson::doc;

// 1. initialize DB if not
// 2. create index on date

pub struct Db(mongodb::Database);

impl Db {

    pub async fn new() -> Result<Self> {
        // Parse a connection string into an options struct.
        let mut client_options = ClientOptions::parse("mongodb://mongo:27017").await?;
        client_options.app_name = Some("Metrics".to_string());

        // Get a handle to the deployment.
        let client = Client::with_options(client_options)?;

        // Get a handle to a database.
        let db = client.database("whackadep");

        //
        Ok(Self(db))
    }

}