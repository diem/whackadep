#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use metrics::{
    model::{Config, Db, Dependencies},
    MetricsRequest,
};
use rocket::State;
use rocket_contrib::json::Json;
use serde::Deserialize;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Mutex;
use std::thread;
use tokio::runtime::Runtime;

//
// Routes
//

#[get("/")]
/// displays all the routes
fn index() -> &'static str {
    // TODO: print other routes?
    "/\n
    /refresh?repo=<REPO>\n
    /dependencies?repo=<REPO>\n
    /repos\n
    /add_repo"
}

#[get("/refresh?<repo>")]
// TODO: does anyhow result implement Responder?
/// starts an analysis for the repo given (if one is not already ongoing)
async fn refresh(state: State<App, '_>, repo: String) -> &'static str {
    // check if we have the repo in our config
    let config = Config::new(state.db.clone());
    match config.repo_exists(&repo).await {
        Ok(true) => (),
        Ok(false) => return "add the repository first",
        Err(e) => {
            error!("{}", e);
            return "error, check the logs";
        }
    };

    // try to request metrics service
    let sender = state.metrics_requester.lock().unwrap();
    if sender
        .try_send(MetricsRequest::StartAnalysis { repo_url: repo })
        .is_err()
    {
        return "metrics service is busy";
    }
    //
    "ok"
}

#[get("/dependencies?<repo>")]
/// obtains latest analysis result for a repository
async fn dependencies(state: State<App, '_>, repo: String) -> String {
    // check if we have the repo in our config
    let config = Config::new(state.db.clone());
    match config.repo_exists(&repo).await {
        Ok(true) => (),
        Ok(false) => return "add the repository first".to_string(),
        Err(e) => {
            error!("{}", e);
            return "error, check the logs".to_string();
        }
    };

    // read from db
    let dependencies = Dependencies::new(state.db.clone());
    match dependencies.get_last_analysis(&repo).await {
        Ok(Some(analysis)) => match serde_json::to_string(&analysis) {
            Ok(dependencies) => return dependencies,
            Err(e) => {
                error!("couldn't serialize dependencies: {}", e);
            }
        },
        Ok(None) => return "no dependency analysis found".to_string(),
        Err(e) => {
            error!(
                "couldn't get dependencies (perhaps a breaking update was applied): {}",
                e
            );
        }
    };
    "an error happened while retrieving dependencies".to_string()
}

#[get("/repos")]
/// obtains latest analysis result for a repository
async fn repos(state: State<App, '_>) -> String {
    let config = Config::new(state.db.clone());
    let repos = config.get_repos().await;
    let repos = match repos {
        Err(e) => return format!("error: {}", e),
        Ok(repos) => repos,
    };
    let repos: Vec<String> = repos.into_iter().map(|repo| repo.repo).collect();
    match serde_json::to_string(&repos) {
        Err(e) => return format!("error: {}", e),
        Ok(repos) => repos,
    }
}

#[derive(Deserialize)]
struct RepoForm {
    repo: String,
}

#[post("/add_repo", format = "json", data = "<repo_form>")]
/// obtains latest analysis result for a repository
async fn add_repo(state: State<App, '_>, repo_form: Json<RepoForm>) -> String {
    // sanitize
    if !valid_repo_url(&repo_form.repo) {
        return "error, the repo url sent is empty".to_string();
    }

    // add to storage
    info!("adding repository: {}", repo_form.repo);
    let config = Config::new(state.db.clone());
    match config.add_new_repo(&repo_form.repo).await {
        Ok(()) => "ok".to_string(),
        Err(e) => format!("error: {}", e),
    }
}

// TODO: complete this function
fn valid_repo_url(repo: &str) -> bool {
    if repo == "" {
        return false;
    }
    true
}

//
// App
//

struct App {
    // to send requests to the metric service
    metrics_requester: Mutex<SyncSender<MetricsRequest>>,
    db: Db,
}

#[launch]
async fn rocket() -> rocket::Rocket {
    // init logging
    tracing_subscriber::fmt::init();
    info!("logging initialized");

    // TODO: run this on the main runtimes
    // start metric server
    let (sender, receiver) = sync_channel::<MetricsRequest>(0);
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(metrics::start(receiver))
            .expect("metrics stopped working");
    });

    // configure app state
    let state = App {
        metrics_requester: Mutex::new(sender),
        db: Db::new(None, None, None, None).await.unwrap(),
    };

    // start server
    info!("starting rocket server");
    rocket::ignite()
        .manage(state)
        .mount("/", routes![index, refresh, dependencies, repos, add_repo])
}
