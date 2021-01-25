//!
//! # Stored structures
//!
//! Note that to remain backward compatible, these structures
//! should only be updated to add field, not remove.
//! (As deserialization of past data wouldn't work anymore.)
//! That being said, we might not store data for very long,
//! so this might not matter...
//!

use anyhow::{anyhow, Context, Result};
use guppy_summaries::{PackageStatus, SummarySource, SummaryWithMetadata};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

mod cratesio;

/// RustAnalysis contains the result of the analysis of a rust workspace
#[derive(Serialize, Deserialize, Default)]
pub struct RustAnalysis {
    /// Note that we do not use a map because the same dependency can be seen several times.
    /// This is due to different versions being used or/and being used directly and indirectly (transitively).
    dependencies: Vec<DependencyInfo>,
}

/// DependencyInfo contains the information obtained from a dependency.
/// Note that some fields might be filled in different stages (e.g. by the priority engine or the risk engine).
#[derive(Serialize, Deserialize)]
pub struct DependencyInfo {
    name: String,
    version: Version,
    repo: SummarySource,
    new_version: Option<NewVersion>,
    dev: bool,
    direct: bool,
}

/// NewVersion should contain any interesting information (red flags, etc.) about the changes observed in the new version
#[derive(Serialize, Deserialize)]
pub struct NewVersion {
    version: Version,
    associated_rustsec: Option<String>,
}

impl RustAnalysis {
    pub fn get_dependencies(repo_dir: &Path) -> Result<Self> {
        let (all_deps, release_deps) = Self::fetch(repo_dir)?;
        Self::filter(all_deps, release_deps)
    }

    fn fetch(repo_dir: &Path) -> Result<(SummaryWithMetadata, SummaryWithMetadata)> {
        println!("running generate-summaries");
        // 1. this will produce a json file containing no dev dependencies
        // (only transitive dependencies used in release)
        let out_dir = tempdir()?;
        let output = Command::new("cargo")
            .current_dir(repo_dir)
            .args(&["x", "generate-summaries"])
            .arg(&out_dir.path())
            .arg("json")
            .output()
            .context("couldn't run cargo x generate-summaries")?;

        if !output.status.success() {
            return Err(anyhow!(
                "cargo x generate-summaries failed: {:?}",
                String::from_utf8(output.stderr)
            ));
        }

        println!("{:?}", out_dir);

        // 2. deserialize the release and the full summary
        println!("deserialize result...");
        let path = out_dir.path().join("summary-release.json");
        let release_deps =
            Self::parse_dependencies(&path).with_context(|| format!("couldn't open {:?}", path))?;

        let path = out_dir.path().join("summary-full.json"); // this will contain the dev dependencies
        let all_deps =
            Self::parse_dependencies(&path).with_context(|| format!("couldn't open {:?}", path))?;

        //
        Ok((all_deps, release_deps))
    }

    /// use guppy summaries
    pub fn filter(
        all_deps: SummaryWithMetadata,
        release_deps: SummaryWithMetadata,
    ) -> Result<Self> {
        println!("filter result...");
        let mut dependencies = Vec::new();

        let all_deps_iter = all_deps
            .target_packages
            .iter()
            .chain(all_deps.host_packages.iter()); // "host" point to build-time dependencies

        for (summary_id, package_info) in all_deps_iter {
            // ignore workspace/internal packages
            if matches!(
                summary_id.source,
                SummarySource::Workspace { .. } | SummarySource::Path { .. }
            ) {
                continue;
            }
            if matches!(
                package_info.status,
                PackageStatus::Initial | PackageStatus::Workspace
            ) {
                continue;
            }

            // dev
            let dev = !release_deps.host_packages.contains_key(summary_id)
                && !release_deps.target_packages.contains_key(summary_id);

            // direct dependency?
            let direct = matches!(package_info.status, PackageStatus::Direct);

            // insert
            dependencies.push(DependencyInfo {
                name: summary_id.name.clone(),
                version: summary_id.version.clone(),
                repo: summary_id.source.clone(),
                new_version: None,
                dev: dev,
                direct: direct,
            });
        }

        //
        Ok(Self { dependencies })
    }

    /// deserialize the release summary
    pub fn parse_dependencies(path: &Path) -> Result<SummaryWithMetadata> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(anyhow::Error::msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_summary() {
        // read the release summary
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/summary-release.json");
        let file = File::open(d).unwrap();
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `User`.
        let u: SummaryWithMetadata = serde_json::from_reader(reader).unwrap();
        println!("{:#?}", u);
    }
}
