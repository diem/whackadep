//! This module abstracts various analysis of source code for a given package

use anyhow::{anyhow, Result};
use camino::Utf8Path;
use guppy::graph::{DependencyDirection, PackageGraph, PackageMetadata};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, fs, iter, path::PathBuf, process::Command};
use tokei::{Config, LanguageType, Languages};

#[derive(Debug, Clone)]
pub struct CodeReport {
    pub name: String,
    pub version: String,
    pub is_direct: bool,
    pub loc_report: Option<LOCReport>,
    pub unsafe_report: Option<UnsafeReport>,
    pub dep_report: Option<DepReport>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct LOCReport {
    pub total_loc: u64, // excludes comment and white lines
    pub rust_loc: u64,  // excludes comment and white lines
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DepReport {
    pub total_deps: u64,
    pub deps_total_loc: u64,
    pub deps_rust_loc: u64,
    pub deps_forbidding_unsafe: u64,
    pub deps_using_unsafe: u64,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct UnsafeReport {
    // Unsafe code used by the cargo geiger
    pub forbids_unsafe: bool,
    pub used_unsafe_count: UnsafeDetails,
    pub unused_unsafe_count: UnsafeDetails,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct UnsafeDetails {
    pub functions: u64,
    pub expressions: u64,
    pub impls: u64,
    pub traits: u64,
    pub methods: u64,
}

pub struct CodeAnalyzer {
    loc_cache: RefCell<HashMap<String, LOCReport>>,
    geiger_cache: RefCell<HashMap<(String, String), GeigerPackageInfo>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeigerReport {
    pub packages: Vec<GeigerPackageInfo>,
    pub used_but_not_scanned_files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeigerPackageInfo {
    pub package: GeigerPackage,
    pub unsafety: Unsafety,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeigerPackage {
    pub id: GeigerPackageId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeigerPackageId {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Unsafety {
    pub used: UnsafeInfo,
    pub unused: UnsafeInfo,
    pub forbids_unsafe: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnsafeInfo {
    pub functions: UnsafeCount,
    pub exprs: UnsafeCount,
    pub item_impls: UnsafeCount,
    pub item_traits: UnsafeCount,
    pub methods: UnsafeCount,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnsafeCount {
    pub safe: u64,
    pub unsafe_: u64,
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self {
            loc_cache: RefCell::new(HashMap::new()),
            geiger_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn analyze_code(self, graph: &PackageGraph) -> Result<Vec<CodeReport>> {
        let mut code_reports: Vec<CodeReport> = Vec::new();

        // Get path to all packages in the workspace
        let package_paths: Vec<&str> = graph
            .workspace()
            .iter()
            .map(|pkg| pkg.manifest_path().as_str())
            .collect();
        // Run Geiger report for each member packages and store result in cache
        // TODO: How to avoid multiple calls for Cargo Geiger and run only once?
        self.get_cargo_geiger_report_for_workspace(package_paths)?;

        // Get direct dependencies
        let direct_dependencies: Vec<PackageMetadata> = graph
            .query_workspace()
            .resolve_with_fn(|_, link| {
                let (from, to) = link.endpoints();
                from.in_workspace() && !to.in_workspace()
            })
            .packages(guppy::graph::DependencyDirection::Forward)
            .filter(|pkg| !pkg.in_workspace())
            .collect();

        for package in &direct_dependencies {
            let loc_report = self.get_loc_report(package.manifest_path())?;
            let unsafe_report =
                self.get_unsafe_report(package.name().to_string(), package.version().to_string())?;

            let dependencies: Vec<PackageMetadata> = graph
                .query_forward(iter::once(package.id()))?
                .resolve()
                .packages(DependencyDirection::Forward)
                .filter(|pkg| pkg.id() != package.id())
                .collect();
            let dep_report = self.get_dep_report(dependencies)?;

            let code_report = CodeReport {
                name: package.name().to_string(),
                version: package.version().to_string(),
                is_direct: true,
                loc_report: Some(loc_report),
                unsafe_report: unsafe_report,
                dep_report: Some(dep_report),
            };

            code_reports.push(code_report);
        }

        Ok(code_reports)
    }

    fn get_dep_report(&self, dependencies: Vec<PackageMetadata>) -> Result<DepReport> {
        let total_deps = dependencies.len() as u64;
        let mut deps_total_loc = 0;
        let mut deps_rust_loc = 0;
        let mut deps_forbidding_unsafe = 0;
        let mut deps_using_unsafe = 0;

        for package in &dependencies {
            let loc_report = self.get_loc_report(package.manifest_path())?;
            deps_total_loc += loc_report.total_loc;
            deps_rust_loc += loc_report.rust_loc;

            let unsafe_report =
                self.get_unsafe_report(package.name().to_string(), package.version().to_string())?;
            if !unsafe_report.is_none() {
                let unsafe_report = unsafe_report.unwrap();
                if unsafe_report.forbids_unsafe {
                    deps_forbidding_unsafe += 1;
                } else {
                    if unsafe_report.used_unsafe_count.expressions > 0 {
                        deps_using_unsafe += 1;
                    }
                }
            }
        }

        Ok(DepReport {
            total_deps,
            deps_total_loc,
            deps_rust_loc,
            deps_forbidding_unsafe,
            deps_using_unsafe,
        })
    }

    fn get_loc_report(&self, manifest_path: &Utf8Path) -> Result<LOCReport> {
        let manifest_path = manifest_path.parent().ok_or_else(|| {
            anyhow!(
                "Cannot find parent directory of Cargo.toml for {}",
                manifest_path
            )
        })?;

        if self.loc_cache.borrow().contains_key(manifest_path.as_str()) {
            let code_report = self
                .loc_cache
                .borrow()
                .get(manifest_path.as_str())
                .ok_or_else(|| anyhow!("Caching error"))?
                .clone();
            return Ok(code_report);
        }

        let paths = &[manifest_path];
        let excluded = &["target"];
        let config = Config {
            hidden: Some(true),
            treat_doc_strings_as_comments: Some(true),
            ..Default::default()
        };

        let mut languages = Languages::new();
        languages.get_statistics(paths, excluded, &config);

        let loc_report = LOCReport {
            total_loc: languages.total().code as u64,
            rust_loc: languages
                .get(&LanguageType::Rust)
                .map(|lang| lang.code as u64)
                .unwrap_or(0),
        };

        // put in loc_cache
        self.loc_cache
            .borrow_mut()
            .insert(manifest_path.to_string(), loc_report);

        let code_report = self
            .loc_cache
            .borrow()
            .get(manifest_path.as_str())
            .ok_or_else(|| anyhow!("Caching error"))?
            .clone();
        Ok(code_report)
    }

    fn get_cargo_geiger_report_for_workspace(&self, package_paths: Vec<&str>) -> Result<()> {
        // Cargo geiger only works with package tomls
        // and not a virtual manifest file
        // Therefore, we run cargo geiger on all member packages
        // TODO: Revisit this design
        let package_paths: Vec<PathBuf> = package_paths
            .iter()
            .map(|path| PathBuf::from(path))
            .collect();

        for path in &package_paths {
            let geiger_report = Self::get_cargo_geiger_report(path)?;
            let geiger_packages = geiger_report.packages;
            for geiger_package in &geiger_packages {
                let package = &geiger_package.package.id;
                let key = (package.name.clone(), package.version.clone());
                if !self.geiger_cache.borrow().contains_key(&key) {
                    // TODO: can the used unsafe code change for separate builds?
                    self.geiger_cache
                        .borrow_mut()
                        .insert(key, geiger_package.clone());
                }
            }
        }

        Ok(())
    }

    fn get_cargo_geiger_report(absolute_path: &PathBuf) -> Result<GeigerReport> {
        let absolute_path = fs::canonicalize(absolute_path)?;
        let absolute_path = absolute_path
            .to_str()
            .ok_or_else(|| anyhow!("error in parsing absolute path for Cargo.toml"))?;

        let output = Command::new("cargo")
            .args(&[
                "geiger",
                "--output-format",
                "Json",
                "--manifest-path",
                absolute_path, // only accepts absolute path
            ])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Error in running cargo geiger"));
        }

        let geiger_report: GeigerReport = serde_json::from_slice(&output.stdout)?;
        Ok(geiger_report)
    }

    fn get_unsafe_report(&self, name: String, version: String) -> Result<Option<UnsafeReport>> {
        let key = (name, version);
        let cache = self.geiger_cache.borrow();

        if !cache.contains_key(&key) {
            // Cargo geiger can not have a result for a valid dependency
            // e.g., openssl not present for geiger report for valid_dep test crate
            return Ok(None);
        }

        let geiger_package_info = cache
            .get(&key)
            .ok_or_else(|| anyhow!("Missing package in Geiger cache"))?;

        Ok(Some(UnsafeReport {
            forbids_unsafe: geiger_package_info.unsafety.forbids_unsafe,
            used_unsafe_count: UnsafeDetails {
                functions: geiger_package_info.unsafety.used.functions.unsafe_,
                expressions: geiger_package_info.unsafety.used.exprs.unsafe_,
                impls: geiger_package_info.unsafety.used.item_impls.unsafe_,
                traits: geiger_package_info.unsafety.used.item_traits.unsafe_,
                methods: geiger_package_info.unsafety.used.methods.unsafe_,
            },
            unused_unsafe_count: UnsafeDetails {
                functions: geiger_package_info.unsafety.unused.functions.unsafe_,
                expressions: geiger_package_info.unsafety.unused.exprs.unsafe_,
                impls: geiger_package_info.unsafety.unused.item_impls.unsafe_,
                traits: geiger_package_info.unsafety.unused.item_traits.unsafe_,
                methods: geiger_package_info.unsafety.unused.methods.unsafe_,
            },
        }))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use guppy::{graph::PackageGraph, MetadataCommand};
    use serial_test::serial;
    use std::path::PathBuf;

    fn get_test_graph() -> PackageGraph {
        MetadataCommand::new()
            .current_dir(PathBuf::from("resources/test/valid_dep"))
            .build_graph()
            .unwrap()
    }

    fn get_test_code_analyzer() -> CodeAnalyzer {
        CodeAnalyzer::new()
    }

    #[test]
    fn test_loc_report_for_valid_package() {
        let graph = get_test_graph();
        let code_analyzer = get_test_code_analyzer();
        let pkg = graph.packages().find(|p| p.name() == "libc").unwrap();
        let report = code_analyzer.get_loc_report(pkg.manifest_path()).unwrap();

        assert!(report.total_loc > 0);
        assert!(report.rust_loc > 0);
    }

    #[test]
    fn test_loc_report_for_non_rust_directory() {
        let non_rust_path = Utf8Path::new("resources/test/non_rust/norust.md");
        let code_analyzer = get_test_code_analyzer();
        let report = code_analyzer.get_loc_report(non_rust_path).unwrap();

        assert_eq!(report.rust_loc, 0);
    }

    #[test]
    fn test_code_dep_report_for_valid_report() {
        let graph = get_test_graph();
        let code_analyzer = get_test_code_analyzer();
        let package = graph.packages().find(|p| p.name() == "octocrab").unwrap();
        let dependencies: Vec<PackageMetadata> = graph
            .query_forward(iter::once(package.id()))
            .unwrap()
            .resolve()
            .packages(DependencyDirection::Forward)
            .filter(|pkg| pkg.id() != package.id())
            .collect();
        let report = code_analyzer.get_dep_report(dependencies).unwrap();

        assert!(report.total_deps > 0);
        assert!(report.deps_total_loc > 0);
        assert!(report.deps_rust_loc > 0);
    }

    #[test]
    #[serial]
    fn test_code_analyzer() {
        let code_analyzer = get_test_code_analyzer();
        let graph = get_test_graph();
        let code_reports = code_analyzer.analyze_code(&graph).unwrap();
        println!("{:?}", code_reports);

        assert!(code_reports.len() > 0);
        let report = &code_reports[0];
        assert_eq!(report.unsafe_report.is_none(), false);
    }

    #[test]
    #[serial]
    #[ignore]
    fn test_code_cargo_geiger() {
        let path = PathBuf::from("resources/test/valid_dep/Cargo.toml");
        let geiger_report = CodeAnalyzer::get_cargo_geiger_report(&path).unwrap();
        println!("{:?}", geiger_report);
        assert!(geiger_report.packages.len() > 0);
    }

    #[test]
    #[serial]
    #[ignore]
    fn test_code_geiger_report_for_workspace() {
        let code_analyzer = get_test_code_analyzer();
        let graph = MetadataCommand::new()
            .current_dir(PathBuf::from(".."))
            .build_graph()
            .unwrap();

        // Get path to all packages in the workspace
        let package_paths: Vec<&str> = graph
            .workspace()
            .iter()
            .map(|pkg| pkg.manifest_path().as_str())
            .collect();

        code_analyzer
            .get_cargo_geiger_report_for_workspace(package_paths)
            .unwrap();
        println!(
            "Total keys in geiger cache: {}",
            code_analyzer.geiger_cache.borrow().len()
        );
        assert!(code_analyzer.geiger_cache.borrow().len() > 0);
    }
}
