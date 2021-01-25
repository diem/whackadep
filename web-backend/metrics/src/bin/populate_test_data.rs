use metrics::db::Db;
use metrics::rust::RustAnalysis;
use mongodb::bson;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    // 1. parse summary file
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("resources/test");

    // 2. deserialize the release and the full summary
    let path = dir.as_path().join("summary-release.json");
    let release_deps = RustAnalysis::parse_dependencies(&path).unwrap();
    let path = dir.as_path().join("summary-full.json");
    let all_deps = RustAnalysis::parse_dependencies(&path).unwrap();

    // 3. filter
    let analysis = RustAnalysis::filter(all_deps, release_deps).unwrap();

    // write bson to db
    let analysis = bson::to_bson(&analysis).unwrap();
    let document = analysis.as_document().unwrap();
    Db::write(document.to_owned()).unwrap();
}
