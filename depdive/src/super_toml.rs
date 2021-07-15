//! This module abstracts various manipulation with Cargo.toml and Cargo.lock files
//! 1. It can create a custom package that repicates the full dependency build of a given workspace
//! TODO: This module can work as a stand-alone crate; isolate and publish

use anyhow::Result;
use camino::Utf8Path;
use guppy::graph::{PackageGraph, PackageMetadata};
use indoc::indoc;
use std::fs::{copy, create_dir_all, File};
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

    pub fn get_super_package_directory(&self, graph: &PackageGraph) -> Result<&TempDir> {
        self.setup_empty_package()?;

        // See if the manifest path has an associated Cargo.lock
        self.copy_cargo_lock_if_exists(graph.workspace().root())?;

        Ok(&self.dir)
    }

    fn copy_cargo_lock_if_exists(&self, workspace_path: &Utf8Path) -> Result<()> {
        if workspace_path.exists() {
            let workspace_lock_path = workspace_path.join("Cargo.lock");
            if workspace_lock_path.exists() {
                // Copy the lock file to super package lock file
                copy(&workspace_lock_path, &self.dir.path().join("Cargo.lock"))?;
            }
        }
        Ok(())
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
    use std::io::{BufRead, BufReader};

    fn get_test_super_package_generator() -> SuperPackageGenerator {
        SuperPackageGenerator::new().unwrap()
    }

    fn get_graph_whackadep() -> PackageGraph {
        MetadataCommand::new().build_graph().unwrap()
    }

    #[test]
    fn test_super_toml_empty_package() {
        let super_package = get_test_super_package_generator();
        super_package.setup_empty_package().unwrap();

        let graph = MetadataCommand::new()
            .manifest_path(&super_package.dir.path().join("Cargo.toml"))
            .build_graph()
            .unwrap();
        assert_eq!(get_direct_dependencies(&graph).len(), 0);
    }

    #[test]
    fn test_super_toml_copy_cargo_lock() {
        let graph = get_graph_whackadep();
        let super_package = get_test_super_package_generator();
        super_package
            .copy_cargo_lock_if_exists(graph.workspace().root())
            .unwrap();


        let super_lock = read_to_string(super_package.dir.path().join("Cargo.lock")).unwrap();
        let lock = read_to_string(graph.workspace().root().join("Cargo.lock")).unwrap();
        assert!(super_lock.eq(&lock));
    }
}
