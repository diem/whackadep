//! This module abstracts various manipulation with Cargo.toml and Cargo.lock files
//! 1. It can create a custom package that repicates the full dependency build of a given workspace
//! TODO: This module can work as a stand-alone crate; isolate and publish

use anyhow::Result;
use guppy::graph::{PackageGraph, PackageMetadata};
use indoc::indoc;
use std::fs::{create_dir_all, File};
use std::io::Write;
use tempfile::{tempdir, TempDir};

/// For a given workspace,
/// This returns a temporary directory of a valid package
/// that replicates the dependency build of a given workspace
/// with a Cargo.toml and Cargo.lock file
pub struct SuperPackageGenerator {
    dir: TempDir,
}

impl SuperPackageGenerator {
    pub fn new() -> Result<Self> {
        Ok(Self { dir: tempdir()? })
    }

    pub fn get_super_package_directory(&self) -> Result<&TempDir> {
        self.setup_empty_package()?;
        Ok(&self.dir)
    }

    fn setup_empty_package(&self) -> Result<()> {
        // Create src directory with main.rs file
        let src = self.dir.path().join("src");
        create_dir_all(&src)?;
        let main = src.join("main.rs");
        let mut output = File::create(&main)?;
        write!(output, "fn main() {{}}")?;

        // Create Cargo.toml file
        let path = self.dir.path().join("Cargo.toml");
        let mut file = File::create(&path)?;
        let toml = self.get_header_string();
        write!(file, "{}", toml).unwrap();

        Ok(())
    }

    fn get_header_string(&self) -> String {
        String::from(indoc! {r#"
                [package]
                name = "super_toml"
                version = "0.1.0"
                edition = "2018"

            "#})
    }
}

// TODO: crate a new module `guppy_wrapper`
// that holds common function like below to be used throughout this crate
fn get_direct_dependencies(graph: &PackageGraph) -> Vec<PackageMetadata> {
    graph
        .query_workspace()
        .resolve_with_fn(|_, link| {
            let (from, to) = link.endpoints();
            from.in_workspace() && !to.in_workspace()
        })
        .packages(guppy::graph::DependencyDirection::Forward)
        .filter(|pkg| !pkg.in_workspace())
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use guppy::MetadataCommand;

    fn get_test_super_package_generator() -> SuperPackageGenerator {
        SuperPackageGenerator::new().unwrap()
    }

    #[test]
    fn test_super_toml_empty_package() {
        let super_package = get_test_super_package_generator();
        super_package.setup_empty_package().unwrap();

        let graph = MetadataCommand::new()
            .manifest_path(&super_package.dir.path().join("Cargo.toml"))
            .build_graph()
            .unwrap();
        assert!(get_direct_dependencies(&graph).is_empty());
    }
}
