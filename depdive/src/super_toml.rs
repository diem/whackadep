//! This module abstracts various manipulation with Cargo.toml and Cargo.lock files
//! 1. It can create a custom package that repicates the full dependency build of a given workspace
//! 2. Check a Cargo.toml is a package or a virtual manifest toml
//! TODO: This module can work as a stand-alone crate; isolate and publish

use anyhow::{anyhow, Result};
use camino::Utf8Path;
use guppy::{
    graph::{DependencyDirection, ExternalSource, PackageGraph, PackageMetadata, PackageSource},
    DependencyKind,
};
use indoc::indoc;
use std::collections::{HashMap, HashSet};
use std::fs::{copy, create_dir_all, read_to_string, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
use tempfile::{tempdir, TempDir};
use toml::Value;
use twox_hash::XxHash64;

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

        // Generate super toml
        self.write_super_toml_dependencies(&graph)?;

        Ok(&self.dir)
    }

    #[cfg(test)]
    fn get_dir(&self) -> &TempDir {
        &self.dir
    }

    fn write_super_toml_dependencies(&self, graph: &PackageGraph) -> Result<()> {
        let mut toml = String::from("\n[dependencies]");

        let deps = get_direct_dependencies(&graph);
        let feature_map = FeatureMapGenerator::get_direct_dependencies_features(&graph)?;

        for dep in &deps {
            let feaure_info = feature_map
                .get(&FeatureMapGenerator::get_featuremap_key_from_packagemetadata(&dep))
                .ok_or_else(|| anyhow!("direct dep {} not found in feature map", dep.name()))?;

            let mut line = format!(
                "\n{} = {{package=\"{}\", version = \"={}\", features =[",
                self.get_unique_name(&dep),
                dep.name(),
                dep.version()
            );

            let features: Vec<String> = feaure_info
                .features
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect();
            line.push_str(&features.join(","));
            line.push(']');

            if !feaure_info.default_feature_enabled {
                line.push_str(" , default_features = false");
            }

            // Handle source path
            match dep.source() {
                PackageSource::External(..) => {
                    if let Some(source) = dep.source().parse_external() {
                        match source {
                            ExternalSource::Registry(..) => (), // TODO: handle non crates.io registries
                            ExternalSource::Git {
                                repository,
                                resolved,
                                ..
                            } => {
                                line.push_str(&format!(
                                    ", git = \"{}\", rev = \"{}\"",
                                    repository, resolved
                                ));
                            }
                            _ => (), // This enum is non-exhaustive
                        }
                    }
                }
                _ => {
                    if let Some(path) = dep.source().local_path() {
                        let absolute_path = graph.workspace().root().join(path);
                        line.push_str(&format!(", path = \"{}\"", absolute_path));
                    }
                }
            }

            line.push('}');

            toml.push_str(&line);
        }
        let path = self.dir.path().join("Cargo.toml");
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        write!(file, "{}", toml).unwrap();

        Ok(())
    }

    fn get_unique_name(&self, dep: &PackageMetadata) -> String {
        let mut hasher = XxHash64::default();
        dep.version().to_string().hash(&mut hasher);
        dep.source().hash(&mut hasher);
        let hash = hasher.finish();
        format!("{}-{:x}", dep.name(), hash)
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

pub struct TomlChecker;

#[derive(Debug, Clone, PartialEq)]
pub enum TomlType {
    Package,
    VirtualManifest,
}

impl TomlChecker {
    fn is_package_toml(path: &Utf8Path) -> Result<bool> {
        let toml = read_to_string(path)?;
        let toml: Value = toml::from_str(&toml)?;
        Ok(toml.get("package").is_some())
    }

    fn is_virtual_manifest_toml(path: &Utf8Path) -> Result<bool> {
        let toml = read_to_string(path)?;
        let toml: Value = toml::from_str(&toml)?;
        Ok(toml.get("package").is_none() && toml.get("workspace").is_some())
    }

    pub fn get_toml_type(path: &Utf8Path) -> Result<TomlType> {
        if !path.ends_with("Cargo.toml") {
            return Err(anyhow!("{} does not point to a Cargo.toml file", path));
        }

        if Self::is_package_toml(&path)? {
            Ok(TomlType::Package)
        } else if Self::is_virtual_manifest_toml(&path)? {
            Ok(TomlType::VirtualManifest)
        } else {
            Err(anyhow!(
                "Cargo.toml is neither package nor workspace. Check format"
            ))
        }
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

#[derive(Debug, Clone, Default)]
struct FeatureInfo {
    default_feature_enabled: bool,
    features: HashSet<String>,
}

struct FeatureMapGenerator;

impl FeatureMapGenerator {
    fn get_direct_dependencies_features(
        graph: &PackageGraph,
    ) -> Result<HashMap<String, FeatureInfo>> {
        let mut feature_map: HashMap<String, FeatureInfo> = HashMap::new();

        // For all workspace members
        for member in graph.packages().filter(|pkg| pkg.in_workspace()) {
            let links = member
                .direct_links_directed(DependencyDirection::Forward)
                .filter(|link| !link.to().in_workspace());
            // For all direct dependencies
            for link in links {
                let key = Self::get_featuremap_key_from_packagemetadata(&link.to());
                if !feature_map.contains_key(&key) {
                    feature_map.insert(key.clone(), FeatureInfo::default());
                }

                let feature_info = feature_map
                    .get_mut(&key)
                    .ok_or_else(|| anyhow!("error in constructing feature map"))?;

                // Not considering dev features
                // TODO: Write dev-dependencies separately in supertoml
                let dep_req_kinds = [DependencyKind::Normal, DependencyKind::Build];
                for req_kind in dep_req_kinds {
                    let dep_req = link.req_for_kind(req_kind);
                    // If default feature is enabled for any, make it true
                    feature_info.default_feature_enabled = feature_info.default_feature_enabled
                        || dep_req.default_features().enabled_on_any();
                    dep_req.features().for_each(|f| {
                        feature_info.features.insert(f.to_string());
                    })
                }
            }
        }

        Ok(feature_map)
    }

    fn get_featuremap_key_from_packagemetadata(package: &PackageMetadata) -> String {
        format!("{}:{}", package.name(), package.version())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::diff::DiffAnalyzer;
    use git2::{build::CheckoutBuilder, Oid};
    use guppy::MetadataCommand;
    use std::path::PathBuf;

    fn get_test_super_package_generator() -> SuperPackageGenerator {
        SuperPackageGenerator::new().unwrap()
    }

    fn get_test_graph_whackadep() -> PackageGraph {
        MetadataCommand::new().build_graph().unwrap()
    }

    fn get_graph_valid_dep() -> PackageGraph {
        MetadataCommand::new()
            .current_dir(PathBuf::from("resources/test/valid_dep"))
            .build_graph()
            .unwrap()
    }

    fn get_all_dependencies(graph: &PackageGraph) -> Vec<PackageMetadata> {
        graph
            .query_workspace()
            .resolve_with_fn(|_, link| !link.to().in_workspace())
            .packages(guppy::graph::DependencyDirection::Forward)
            .filter(|pkg| !pkg.in_workspace())
            .collect()
    }

    fn assert_super_package_equals_graph(graph: &PackageGraph) {
        let super_package = get_test_super_package_generator();
        let dir = super_package.get_super_package_directory(&graph).unwrap();

        let super_graph = MetadataCommand::new()
            .manifest_path(dir.path().join("Cargo.toml"))
            .build_graph()
            .unwrap();
        assert_eq!(
            get_all_dependencies(&graph).len(),
            get_all_dependencies(&super_graph).len()
        );

        let mut hs: HashSet<(String, semver::Version)> = HashSet::new();
        for dep in &get_all_dependencies(&graph) {
            hs.insert((dep.name().to_string(), dep.version().clone()));
        }

        let mut super_hs: HashSet<(String, semver::Version)> = HashSet::new();
        for dep in &get_all_dependencies(&super_graph) {
            super_hs.insert((dep.name().to_string(), dep.version().clone()));
        }

        assert!(hs.len() == super_hs.len() && hs.iter().all(|k| super_hs.contains(k)));
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
        let graph = get_test_graph_whackadep();
        let super_package = get_test_super_package_generator();
        super_package
            .copy_cargo_lock_if_exists(graph.workspace().root())
            .unwrap();

        let super_lock = read_to_string(super_package.dir.path().join("Cargo.lock")).unwrap();
        let lock = read_to_string(graph.workspace().root().join("Cargo.lock")).unwrap();
        assert!(super_lock.eq(&lock));
    }

    #[test]
    fn test_super_toml_feature_map() {
        let graph = get_test_graph_whackadep();
        let direct_deps = get_direct_dependencies(&graph);
        let feature_map = FeatureMapGenerator::get_direct_dependencies_features(&graph).unwrap();
        assert_eq!(direct_deps.len(), feature_map.len());
    }

    #[test]
    fn test_super_toml_package() {
        assert_super_package_equals_graph(&get_graph_valid_dep());
        assert_super_package_equals_graph(&get_test_graph_whackadep())
    }

    #[test]
    fn test_super_toml_type() {
        assert_eq!(
            TomlChecker::get_toml_type(Utf8Path::new("resources/test/valid_dep/Cargo.toml"))
                .unwrap(),
            TomlType::Package
        );

        assert_eq!(
            TomlChecker::get_toml_type(Utf8Path::new("../Cargo.toml")).unwrap(),
            TomlType::VirtualManifest
        );
    }

    #[test]
    fn test_super_toml_invlaid_cargo_toml() {
        assert!(TomlChecker::get_toml_type(Utf8Path::new("../Cargo.lock")).is_err());
    }

    #[test]
    fn test_super_toml_cargo_lock() {
        let da = DiffAnalyzer::new().unwrap();
        let repo = da
            .get_git_repo("whackadep", "https://github.com/diem/whackadep")
            .unwrap();
        let mut checkout_builder = CheckoutBuilder::new();
        checkout_builder.force();
        repo.checkout_tree(
            &repo
                .find_object(
                    Oid::from_str("6b50de6941e00c3cd7fd34c1bf64793f514f434f").unwrap(),
                    None,
                )
                .unwrap(),
            Some(&mut checkout_builder),
        )
        .unwrap();
        let path = repo.path().parent().unwrap();
        let graph = MetadataCommand::new()
            .current_dir(path)
            .build_graph()
            .unwrap();
        assert_super_package_equals_graph(&graph);

        // Now test without copying the cargo lock which should fail
        let super_package = get_test_super_package_generator();
        super_package.setup_empty_package().unwrap();
        super_package.write_super_toml_dependencies(&graph).unwrap();
        let dir = super_package.get_dir();

        let super_graph = MetadataCommand::new()
            .manifest_path(dir.path().join("Cargo.toml"))
            .build_graph()
            .unwrap();
        // Dep graph won't be the same as a dep graph generated from the Cargo.Toml
        // won't match an old Cargo.lock
        assert_ne!(
            get_all_dependencies(&graph).len(),
            get_all_dependencies(&super_graph).len()
        );
        let mut hs: HashSet<(String, semver::Version)> = HashSet::new();
        for dep in &get_all_dependencies(&graph) {
            hs.insert((dep.name().to_string(), dep.version().clone()));
        }
        let mut super_hs: HashSet<(String, semver::Version)> = HashSet::new();
        for dep in &get_all_dependencies(&super_graph) {
            super_hs.insert((dep.name().to_string(), dep.version().clone()));
        }
        assert!(!(hs.len() == super_hs.len() && hs.iter().all(|k| super_hs.contains(k))));
    }

    #[test]
    fn test_suoer_toml_on_diem() {
        // TODO: replace diem, whackadep, valid_dep with a test repo that involves
        // all different challenges a virtual manifest can present
        let da = DiffAnalyzer::new().unwrap();
        let repo = da
            .get_git_repo("diem", "https://github.com/diem/diem.git")
            .unwrap();
        let path = repo.path().parent().unwrap();
        let graph = MetadataCommand::new()
            .current_dir(path)
            .build_graph()
            .unwrap();
        assert_super_package_equals_graph(&graph);
    }
}
