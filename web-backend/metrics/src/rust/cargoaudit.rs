//! There are different types of results in Cargo-audit
//! The important distinction is:
//! - there is no patch
//! - there are versions that are unaffected

use anyhow::{ensure, Context, Result};
use rustsec::{advisory::Informational, lockfile::Lockfile, registry, warning, Report, Warning};
use std::path::Path;
use tokio::process::Command;
use tracing::info;

/// performs an audit of the Cargo.lock file with rustsec
pub async fn audit(repo_path: &Path) -> Result<Report> {
    // config
    let advisory_db_url = rustsec::repository::git::DEFAULT_URL;
    // TODO: do we want to use a custom path here?
    let advisory_db_path = rustsec::repository::git::Repository::default_path();

    // TODO: rm -rf advisory_db_path
    // rationale: if once the command fails or get interrupted, the path gets damaged, and fetch fails afterwards everytime
    // https://github.com/RustSec/rustsec/issues/32

    // fetch latest changes from the advisory + load
    info!("fetching latest version of RUSTSEC advisory...");
    let advisory_db_repo =
        rustsec::repository::git::Repository::fetch(advisory_db_url, &advisory_db_path, true)
            .with_context(|| "couldn't fetch RUSTSEC advisory database")?;
    let advisory_db = rustsec::Database::load_from_repo(&advisory_db_repo)
        .with_context(|| "couldn't open RUSTSEC repo")?;

    // make sure a Carg.lock file is there
    generate_lockfile(repo_path).await?;

    // open Cargo.lock file
    let lockfile_path = repo_path.join("Cargo.lock");
    let lockfile = Lockfile::load(&lockfile_path)?;

    // run audit
    info!("generating rustsec report...");
    let mut settings = rustsec::report::Settings::default();
    settings.informational_warnings = vec![
        Informational::Unmaintained,
        Informational::Notice,
        Informational::Unsound,
    ]; // these are the only three informational advisories at the moment
    info!("settings: {:#?}", settings);
    let mut report = rustsec::Report::generate(&advisory_db, &lockfile, &settings);

    // check for yanked versions as well
    // TODO: move this elsewhere in priority engine? (especially as we are not leveraging guppy's results here)
    info!("fetching latest crates.io index to check for yanked versions...");
    let registry_index = registry::Index::fetch()?; // refresh crates.io index

    info!("finding yanked versions...");
    use std::collections::btree_map::Entry;
    for package in &lockfile.packages {
        if let Ok(pkg) = registry_index.find(&package.name, &package.version) {
            if pkg.is_yanked {
                let warning = Warning::new(warning::Kind::Yanked, package, None, None);
                match report.warnings.entry(warning::Kind::Yanked) {
                    Entry::Occupied(entry) => (*entry.into_mut()).push(warning),
                    Entry::Vacant(entry) => {
                        entry.insert(vec![warning]);
                    }
                }
            }
        }
    }

    //
    info!("rustsec audit done");
    Ok(report)
}

pub async fn generate_lockfile(repo_path: &Path) -> Result<()> {
    let output = Command::new("cargo")
        .current_dir(repo_path)
        .arg("generate-lockfile")
        .output()
        .await?;

    ensure!(
        output.status.success(),
        "couldn't run cargo generate-lockfile: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}
