use metrics::db::Db;

#[tokio::main]
async fn main() {
  let db = Db::new().await.unwrap();
}
