#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use anyhow::Error;
use rocket::State;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Mutex;
use std::thread;

mod metrics;

use crate::metrics::MetricsRequest;

//
// Routes
//

#[get("/")]
fn index() -> &'static str {
    // TODO: print other routes?
    "/metrics"
}

#[get("/metrics")]
// TODO: does anyhow result implement Responder?
fn metrics(state: State<App>) -> &'static str {
    let sender = state.metrics_requester.lock().unwrap();
    if sender.try_send(MetricsRequest::Dependencies).is_err() {
        return "metrics service is busy";
    }
    //
    "ok"
}

//
// App
//

struct App {
    // to send requests to the metric service
    metrics_requester: Mutex<SyncSender<MetricsRequest>>,
}

fn main() {
    // start metric server
    let (sender, receiver) = sync_channel::<MetricsRequest>(1);
    thread::spawn(move || {
        metrics::start(receiver).expect("metrics stopped working");
    });

    // configure app state
    let state = App {
        metrics_requester: Mutex::new(sender),
    };

    // start server
    rocket::ignite()
        .manage(state)
        .mount("/", routes![index, metrics])
        .launch();
}
