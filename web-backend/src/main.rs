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
    if sender.try_send(MetricsRequest::Dependencies).is_err() {
        return "metrics service is busy";
    }
    //
    "ok"
}

#[get("/dependencies")]
fn dependencies() -> String {
    let mut rt = Runtime::new().unwrap();

    let db = match rt.block_on(Db::new()) {
        Ok(db) => db,
        Err(_) => return "couldn't connect to the database".to_string(),
    };
    let dependencies = rt.block_on(db.get_dependencies());
    match dependencies {
        Ok(dependencies) => match serde_json::to_string(&dependencies) {
            Ok(dependencies) => dependencies,
            Err(_) => "couldn't deserialize dependencies".to_string(),
        },
        Err(_) => "couldn't find any dependencies".to_string(),
    }
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
    // start metric server
    let (sender, receiver) = sync_channel::<MetricsRequest>(0);
    thread::spawn(move || {
        let mut rt = Runtime::new().unwrap();
        rt.block_on(metrics::start(receiver))
            .expect("metrics stopped working");
    });

    // configure app state
    let state = App {
        metrics_requester: Mutex::new(sender),
    };

    // start server
    rocket::ignite()
        .manage(state)
        .mount("/", routes![index, refresh, dependencies])
}
