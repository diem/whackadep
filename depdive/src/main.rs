use anyhow::Result;
use depdive::{DependencyAnalyzer, UpdateAnalyzer};
use std::path::Path;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Rust dependency analysis")]
struct Args {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "update-review")]
    /// Return a review of the dep updates
    /// between a prior and post state
    UpdateReview {
        #[structopt(subcommand)]
        cmd: UpdateReviewCommand,
    },

    #[structopt(name = "dep-review")]
    /// Returns usage and activity metrics in Json
    /// for all dependencies of a cargo project
    DepReview {
        #[structopt(subcommand)]
        cmd: DepReviewCommand,
    },
}

#[derive(Debug, StructOpt)]
enum DepReviewCommand {
    #[structopt(name = "package-metrics")]
    PackageMetrics {
        /// Path to repo
        path: String,
        #[structopt(long)]
        /// if only direct deps should be analyzed
        only_direct: Option<bool>,
    },
}

#[derive(Debug, StructOpt)]
enum UpdateReviewCommand {
    #[structopt(name = "paths")]
    Paths {
        /// Path to repository old state
        prior: String,
        /// Path to repository new state post update
        post: String,
    },

    #[structopt(name = "commits")]
    Commits {
        /// Path to the git repository
        path: String,
        /// Commit sha prior to update
        prior: String,
        /// Commit sha post update
        post: String,
    },
}

fn update_analyzer_from_paths(prior: &str, post: &str) -> Result<()> {
    let report = UpdateAnalyzer::run_update_analyzer_from_paths(Path::new(prior), Path::new(post))?
        .unwrap_or_default();
    println!("{}", report);
    Ok(())
}

fn update_analyzer_from_repo_commits(
    path: &str,
    prior_commit: &str,
    post_commit: &str,
) -> Result<()> {
    let report = UpdateAnalyzer::run_update_analyzer_from_repo_commits(
        Path::new(path),
        prior_commit,
        post_commit,
    )?
    .unwrap_or_default();
    println!("{}", report);
    Ok(())
}

fn get_package_metrics_for_deps_in_json(path: &str, only_direct: Option<bool>) -> Result<()> {
    let report = DependencyAnalyzer::get_dep_pacakge_metrics_in_json_from_path(
        Path::new(path),
        only_direct.unwrap_or(false),
    )?;
    println!("{}", report);
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::from_iter(std::env::args());

    match args.cmd {
        Command::UpdateReview { cmd } => match cmd {
            UpdateReviewCommand::Paths { prior, post } => update_analyzer_from_paths(&prior, &post),
            UpdateReviewCommand::Commits { path, prior, post } => {
                update_analyzer_from_repo_commits(&path, &prior, &post)
            }
        },
        Command::DepReview { cmd } => match cmd {
            DepReviewCommand::PackageMetrics { path, only_direct } => {
                get_package_metrics_for_deps_in_json(&path, only_direct)
            }
        },
    }
}
