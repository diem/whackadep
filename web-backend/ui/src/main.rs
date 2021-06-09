use anyhow::Result;
use metrics::{
    model::{Config, Db, Dependencies},
    MetricsRequest,
};
use serde::Deserialize;
use std::{
    net::{IpAddr, SocketAddr},
    sync::{
        mpsc::{sync_channel, SyncSender},
        Arc, Mutex,
    },
    thread,
};
use tokio::runtime::Runtime;
use tokio::time;
use tracing::{error, info};
use warp::Filter;

//
// App
//

#[derive(Clone)]
struct App {
    // to send requests to the metric service
    metrics_requester: Arc<Mutex<SyncSender<MetricsRequest>>>,
    db: Db,
}

//
// Routes
//

/// displays all the routes
fn index() -> &'static str {
    // TODO: print other routes?
    "/\n\
    /refresh?repo=<REPO>\n\
    /dependencies?repo=<REPO>\n\
    /repos\n\
    /add_repo"
}

#[derive(Deserialize)]
struct RefreshQuery {
    repo: String,
}

/// starts an analysis for the repo given (if one is not already ongoing)
async fn refresh(app: App, query: RefreshQuery) -> Result<&'static str, warp::Rejection> {
    // check if we have the repo in our config
    let config = Config::new(app.db.clone());
    match config.repo_exists(&query.repo).await {
        Ok(true) => (),
        Ok(false) => return Ok("add the repository first"),
        Err(e) => {
            error!("{}", e);
            return Ok("error, check the logs");
        }
    };

    // try to request metrics service
    let message = match request_metric_analysis(app, query.repo).await {
        Ok(_success) => "ok",
        Err(_error) => "metrics service is busy",
    };

    Ok(message)
}

#[derive(Deserialize)]
struct DependenciesQuery {
    repo: String,
}

/// obtains latest analysis result for a repository
async fn dependencies(app: App, query: DependenciesQuery) -> Result<String, warp::Rejection> {
    // check if we have the repo in our config
    let config = Config::new(app.db.clone());
    match config.repo_exists(&query.repo).await {
        Ok(true) => (),
        Ok(false) => return Ok("add the repository first".to_string()),
        Err(e) => {
            error!("{}", e);
            return Ok("error, check the logs".to_string());
        }
    };

    // read from db
    let dependencies = Dependencies::new(app.db.clone());
    match dependencies.get_last_analysis(&query.repo).await {
        Ok(Some(analysis)) => match serde_json::to_string(&analysis) {
            Ok(dependencies) => return Ok(dependencies),
            Err(e) => {
                error!("couldn't serialize dependencies: {}", e);
            }
        },
        Ok(None) => return Ok("no dependency analysis found".to_string()),
        Err(e) => {
            error!(
                "couldn't get dependencies (perhaps a breaking update was applied): {}",
                e
            );
        }
    };
    Ok("an error happened while retrieving dependencies".to_string())
}

/// obtains latest analysis result for a repository
async fn repos(app: App) -> Result<String, warp::Rejection> {
    let repos = match get_all_repos(app).await {
        Err(e) => return Ok(format!("error: {}", e)),
        Ok(repos) => repos,
    };
    match serde_json::to_string(&repos) {
        Err(e) => Ok(format!("error: {}", e)),
        Ok(repos) => Ok(repos),
    }
}

#[derive(Deserialize)]
struct RepoForm {
    repo: String,
}

/// obtains latest analysis result for a repository
async fn add_repo(app: App, repo_form: RepoForm) -> Result<String, warp::Rejection> {
    // sanitize
    if !valid_repo_url(&repo_form.repo) {
        return Ok("error, the repo url sent is empty".to_string());
    }

    // add to storage
    info!("adding repository: {}", repo_form.repo);
    let config = Config::new(app.db.clone());
    Ok(match config.add_new_repo(&repo_form.repo).await {
        Ok(()) => "ok".to_string(),
        Err(e) => format!("error: {}", e),
    })
}

// TODO: complete this function
fn valid_repo_url(repo: &str) -> bool {
    if repo.is_empty() {
        return false;
    }
    true
}

// Fetches the list of exisiting repositories in analysis database
async fn get_all_repos(app: App) -> Result<Vec<String>> {
    let config = Config::new(app.db.clone());
    let repos = match config.get_repos().await {
        Ok(repos) => repos,
        Err(error) => return Err(error.into()),
    };
    let repos: Vec<String> = repos.into_iter().map(|repo| repo.repo).collect();
    Ok(repos)
}

async fn request_metric_analysis(app: App, repo: String) -> Result<()> {
    // try to request metrics service
    let sender = app.metrics_requester.lock().unwrap();
    let sender = sender.try_send(MetricsRequest::StartAnalysis { repo_url: repo })?;
    Ok(sender)
}

// Periodically kicks off analysis for all repo
async fn cron_job(app: App) {
    let analysis_interval = 1 * 60 * 60; // 1 hour
    let mut interval = time::interval(time::Duration::from_secs(analysis_interval));

    loop {
        interval.tick().await;

        let repos: Vec<String> = match get_all_repos(app.clone()).await {
            Ok(repos) => repos,
            Err(_error) => vec![],
        };
        info!("{} repository(s) to be analyzed", repos.len());

        for repo in &repos {
            match request_metric_analysis(app.clone(), repo.clone()).await {
                Ok(_message) => {
                    info!("Periodic analysis started successfully for {}", repo);
                }
                Err(_error) => {
                    info!("analysis delayed for {}: metric servic is busy", repo);
                }
            };
        }
    }
}

#[tokio::main]
async fn main() {
    // init logging
    tracing_subscriber::fmt::init();
    info!("logging initialized");
    let address = std::env::var("ROCKET_ADDRESS")
        .as_deref()
        .unwrap_or("127.0.0.1")
        .parse::<IpAddr>()
        .unwrap();
    let port = std::env::var("ROCKET_PORT")
        .as_deref()
        .unwrap_or("8080")
        .parse::<u16>()
        .unwrap();

    // TODO: run this on the main runtimes
    // start metric server
    let (sender, receiver) = sync_channel::<MetricsRequest>(0);
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(metrics::start(receiver))
            .expect("metrics stopped working");
    });

    // configure app state
    let app = App {
        metrics_requester: Arc::new(Mutex::new(sender)),
        db: Db::new(None, None, None, None).await.unwrap(),
    };

    // kicks off a cron job in a thread
    tokio::spawn(cron_job(app.clone()));

    let app = warp::any().map(move || app.clone());

    //
    // Routes
    //

    // GET /
    let index = warp::get().and(warp::path::end()).map(index);

    // GET /refresh?<repo>
    let refresh = warp::get()
        .and(warp::path("refresh"))
        .and(app.clone())
        .and(warp::query::<RefreshQuery>())
        .and_then(refresh);

    // GET /dependencies?<repo>
    let dependencies = warp::get()
        .and(warp::path("dependencies"))
        .and(app.clone())
        .and(warp::query::<DependenciesQuery>())
        .and_then(dependencies);

    // GET /repos
    let repos = warp::get()
        .and(warp::path("repos"))
        .and(app.clone())
        .and_then(repos);

    // POST /add_repo
    let add_repo = warp::post()
        .and(warp::path("add_repo"))
        .and(app.clone())
        .and(warp::body::json())
        .and_then(add_repo);

    let routes = index
        .or(refresh)
        .or(dependencies)
        .or(repos)
        .or(add_repo)
        .with(warp::log("requests"))
        .with(warp::trace::request());

    warp::serve(routes)
        .run(SocketAddr::new(address, port))
        .await
}
