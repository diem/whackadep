#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use metrics::{
    model::{Db, Dependencies},
    MetricsRequest,
};
use rocket::State;
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
    "/refresh?repo=<REPO>\n/dependencies?repo=<REPO>"
}

#[get("/refresh?<repo>")]
// TODO: does anyhow result implement Responder?
/// starts an analysis for the repo given (if one is not already ongoing)
fn refresh(state: State<App>, repo: String) -> &'static str {
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
            error!("couldn't get dependencies: {}", e);
        }
    };
    "an error happened while retrieving dependencies".to_string()
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
        .mount("/", routes![index, refresh, dependencies])
}
