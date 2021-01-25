use anyhow::Result;
use futures::{stream, StreamExt};

/*
use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldResult, GraphQLEnum, Variables,
};
use reqwest::Client;
use tokio;

// TODO: get dependabot alert (https://docs.github.com/en/graphql/reference/objects#repositoryvulnerabilityalert)

pub async fn get_dependabot_alerts(repo: &str) {
    // Create a context object.
    let ctx = Ctx(Episode::NewHope);

    // Run the executor.
    let (res, _errors) = juniper::execute_sync(
        "query { favoriteEpisode }",
        None,
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &Variables::new(),
        &ctx,
    )
    .unwrap();

    // Ensure the value matches.
    assert_eq!(
        res,
        graphql_value!({
            "favoriteEpisode": "NEW_HOPE",
        })
    );
}
*/

/*
const CONCURRENT_REQUESTS: usize = 10;

pub async fn get(urls: Vec<&str>) {
    let client = Client::new();

    let bodies = stream::iter(urls)
        .map(|url| {
            let client = &client;
            async move {
                let resp = client.get(url).send().await?;
                resp.bytes().await
            }
        })
        .buffer_unordered(CONCURRENT_REQUESTS);

    bodies
        .for_each(|b| async {
            match b {
                Ok(b) => println!("Got {} bytes", b.len()),
                Err(e) => eprintln!("Got an error: {}", e),
            }
        })
        .await;
}
*/

/// e.g. repo: "diem/diem"
// what's interesting there?
// - stargazers_count
pub async fn get_repository_info(repo: &str) -> Result<octocrab::models::Repository> {
    octocrab::instance()
        .get(
            format!("https://api.github.com/repos/{}", repo),
            None::<&()>,
        )
        .await
        .map_err(anyhow::Error::msg)
}
