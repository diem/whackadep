#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::State;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::thread;

mod metrics;

//
// Routes
//

#[get("/")]
fn index() -> &'static str {
    // TODO: print other routes?
    "Hello, world!"
}

#[get("/metrics")]
fn metrics(state: State<App>) -> &'static str {
    let sender = state.metrics_requester.lock().unwrap();
    sender.send("hello!".to_string());
    return "ok";
}

//
// App
//

struct App {
    // to send requests to the metric service
    metrics_requester: Mutex<Sender<String>>,
}

fn main() {
    // start metric server
    let (sender, receiver) = channel::<String>();
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
