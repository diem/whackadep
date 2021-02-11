use anyhow::Result;
use guppy::{
    graph::{
        cargo::{CargoOptions, CargoResolverVersion},
        feature::StandardFeatures,
        summaries::Summary,
        PackageGraph,
    },
    MetadataCommand,
};
use std::path::Path;
use target_spec::{Platform, TargetFeatures};
use tracing::{debug, info};

/// Obtains all dependencies (normal/build/dev and direct/transitive)
/// that get imported when default features are used.
pub fn get_guppy_summaries(manifest_path: &Path) -> Result<(Summary, Summary)> {
    info!("obtaining dependencies from {:?}", manifest_path);
    let no_dev_summary = get_dependencies_inner(manifest_path, false)?;
    let all_summary = get_dependencies_inner(manifest_path, true)?;
    //
    Ok((no_dev_summary, all_summary))
}

/// Obtains all dependencies (normal/build/dev and direct/transitive)
/// that get imported when default features are used.
pub fn get_dependencies_inner(manifest_path: &Path, include_dev: bool) -> Result<Summary> {
    // obtain metadata from manifest_path
    let mut cmd = MetadataCommand::new();
    cmd.manifest_path(manifest_path);

    // construct graph with guppy
    let package_graph = PackageGraph::from_command(&mut cmd).map_err(anyhow::Error::msg)?;

    // cargo options
    let mut opts = CargoOptions::new();
    // TODO: do we have to switch to v2 when rust will do the change?
    opts.set_version(CargoResolverVersion::V1)
        .set_include_dev(include_dev);

    // we're simulating a build on all workspace crates
    let package_set = package_graph.resolve_workspace();
    let feature_set = package_set.to_feature_set(StandardFeatures::Default); // standard cargo build
    let cargo_set = feature_set.into_cargo_set(&opts)?;

    // produce summary
    let summary = cargo_set.to_summary(&opts)?;
    debug!("summary obtained: {:?}", summary);

    //
    Ok(summary)
}

/// Obtains all dependencies (normal/build/dev and direct/transitive)
/// that get imported when default features are used.
pub fn get_dependencies_inner_custom(
    manifest_path: &Path,
    include_dev: bool,
    v2resolver: bool,
    features: Vec<&str>,
    platform_triplet: &str,
    ignored_packages: Vec<&str>,
) -> Result<Summary> {
    // obtain metadata from manifest_path
    let mut cmd = MetadataCommand::new();
    cmd.manifest_path(manifest_path);

    // construct graph with guppy
    let package_graph = PackageGraph::from_command(&mut cmd).map_err(anyhow::Error::msg)?;

    // cargo options
    let mut opts = CargoOptions::new();

    let target_features = TargetFeatures::Unknown;
    let platform = Platform::new(platform_triplet, target_features)?;
    opts.set_platform(Some(platform));

    let resolver = if v2resolver {
        CargoResolverVersion::V2
    } else {
        CargoResolverVersion::V1
    };
    opts.set_version(resolver).set_include_dev(include_dev);

    // we're simulating a build on all workspace crates
    let package_set = package_graph.resolve_workspace();
    let feature_set = package_set.to_feature_set(StandardFeatures::Default); // standard cargo build
    let cargo_set = feature_set.into_cargo_set(&opts)?;

    // produce summary
    let summary = cargo_set.to_summary(&opts)?;
    debug!("summary obtained: {:?}", summary);

    //
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::Repo;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_on_dephell() {
        let mut manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_path.push("resources/test/sample_repo/Cargo.toml");

        let summary = get_dependencies_inner(&manifest_path, true).unwrap();

        println!("{:#?}", summary);
        assert!(summary
            .target_packages
            .iter()
            .find(|p| p.0.name == "bitvec")
            .is_none());
        assert!(summary
            .target_packages
            .iter()
            .find(|p| p.0.name == "optional_dep")
            .is_some());
    }
}
