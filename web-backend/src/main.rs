#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use metrics::{db::Db, MetricsRequest};
use rocket::State;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Mutex;
use std::thread;
use tokio::runtime::Runtime;

//
// Routes
//

#[get("/")]
fn index() -> &'static str {
    // TODO: print other routes?
    "/refresh\n/dependencies"
}

#[get("/refresh")]
// TODO: does anyhow result implement Responder?
fn refresh(state: State<App>) -> &'static str {
    let sender = state.metrics_requester.lock().unwrap();
    if sender
        .try_send(MetricsRequest::RustDependencies {
            repo_url: "https://github.com/diem/diem.git".to_string(),
        })
        .is_err()
    {
        return "metrics service is busy";
    }
    //
    "ok"
}

#[get("/dependencies")]
fn dependencies() -> String {
    match Db::get_last_analysis() {
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
}

#[launch]
fn rocket() -> rocket::Rocket {
    // init logging
    tracing_subscriber::fmt::init();
    info!("logging initialized");

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
    };

    // start server
    info!("starting rocket server");
    rocket::ignite()
        .manage(state)
        .mount("/", routes![index, refresh, dependencies])
}
