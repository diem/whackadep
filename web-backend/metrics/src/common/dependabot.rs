//! This module abstract the [dependabot](https://github.com/dependabot/dependabot-core/) library.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::error;

#[derive(Deserialize, Default, Serialize, Debug, PartialEq, Clone)]
pub struct UpdateMetadata {
    changelog_url: Option<String>,
    // TODO: #[serde(skip)]
    changelog_text: Option<String>,
    commits_url: Option<String>,
    // TODO: #[serde(skip)]
    commits: Vec<Commit>,
}

#[derive(Deserialize, Default, Serialize, Debug, PartialEq, Clone)]
pub struct Commit {
    message: String,
    html_url: String,
}

pub async fn get_update_metadata(
    package_manager: &str,
    package: &str,
    version: &str,
    new_version: &str,
) -> Result<UpdateMetadata> {
    let mut dependabot_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dependabot_dir.push("dependabot");

    let output = Command::new("ruby")
        .current_dir(dependabot_dir)
        .env("DEPENDABOT_PACKAGE_MANAGER", package_manager)
        .env("DEPENDABOT_PACKAGE", package)
        .env("DEPENDABOT_VERSION", version)
        .env("DEPENDABOT_NEW_VERSION", new_version)
        .arg("changelog.rb")
        .output()
        .await?;

    if !output.status.success() {
        bail!(
            "couldn't run dependabot: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let data = serde_json::from_slice(&output.stdout).map_err(|e| {
        error!("{}", String::from_utf8_lossy(&output.stdout));
        anyhow::Error::msg(e)
    });

    parse_texts(data)
}

// parse_texts will parse the commits and the changelog in order to retain only these where certain
// words are mentioned that could indicate a security issue. We discard the rest of the data in
// order to reduce the database size.
fn parse_texts(input: Result<UpdateMetadata>) -> Result<UpdateMetadata> {
    if input.is_err() {
        return input;
    }
    if std::env::var("RETAIN_ALL").is_ok() &&  std::env::var("RETAIN_ALL").unwrap() != ""  {
        println!("DISABLED TEXT PARSING, RETAINING ALL DATA. $RETAIN_ALL={}", std::env::var("RETAIN_ALL").unwrap());
        return input;
    }

    let mut data = input.unwrap();
    match data.changelog_text.as_mut() {
        Some(text) => if !flagged_text(text) {
            *text = "".to_string();
        },
        None => {}
    }

     /*
     // This is if we want to retain all commits, deleting the uninteresting messages only
     for commit in data.commits.iter_mut(){
        if flagged_text(&commit.message) {
            println!("FLAGGED COMMIT!: {}", commit.message);
        } else {
            commit.message = "".to_string();
        }
    }
    */

    // This removes completely the commits that are deemed uninteresting
    data.commits.retain(|x| flagged_text(&x.message));

    Ok(data)
}

const FLAGGED_WORDS: &'static [&'static str] = &["bug", "secur", "critical",
                                                "crash", "seed", "key", "malicious",
                                                "overflow", "underflow", "sec",
                                                "severity", "sev", "unsafe",
                                                "secret", "hash", "encrypt",
                                                "exploit","attack", "defense",
                                                "vuln","dos","denial", "rce",
                                                "code exec", "CVE", "advisory",
                                                "hack", "crack", "brute", "harden",
                                                "injection", "hijack", "elevation",
                                                "privilege"];

fn flagged_text(text: &String) -> bool {
    for word in FLAGGED_WORDS {
        if text.contains(word) {
            return true;
        }
    }
    return false;
}