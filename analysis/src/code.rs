//! This module abstracts various analysis of source code for a given package

use anyhow::{anyhow, Result};
use camino::Utf8Path;
use guppy::graph::{DependencyDirection, PackageGraph, PackageMetadata};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, iter};
use tokei::{Config, LanguageType, Languages};

#[derive(Debug, Clone)]
pub struct CodeReport {
    pub name: String,
    pub version: String,
    pub is_direct: bool,
    pub loc_report: Option<LOCReport>,
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
}

pub struct CodeAnalyzer {
    cache: RefCell<HashMap<String, LOCReport>>,
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn analyze_code(self, graph: &PackageGraph) -> Result<Vec<CodeReport>> {
        let mut code_reports: Vec<CodeReport> = Vec::new();

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

        for package in &dependencies {
            let loc_report = self.get_loc_report(package.manifest_path())?;
            deps_total_loc += loc_report.total_loc;
            deps_rust_loc += loc_report.rust_loc;
        }

        Ok(DepReport {
            total_deps,
            deps_total_loc,
            deps_rust_loc,
        })
    }

    fn get_loc_report(&self, manifest_path: &Utf8Path) -> Result<LOCReport> {
        let manifest_path = manifest_path.parent().ok_or_else(|| {
            anyhow!(
                "Cannot find parent directory of Cargo.toml for {}",
                manifest_path
            )
        })?;

        if self.cache.borrow().contains_key(manifest_path.as_str()) {
            let code_report = self
                .cache
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

        // put in cache
        self.cache
            .borrow_mut()
            .insert(manifest_path.to_string(), loc_report);

        let code_report = self
            .cache
            .borrow()
            .get(manifest_path.as_str())
            .ok_or_else(|| anyhow!("Caching error"))?
            .clone();
        Ok(code_report)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use guppy::{graph::PackageGraph, MetadataCommand};
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
    fn test_code_analyzer() {
        let code_analyzer = get_test_code_analyzer();
        let graph = get_test_graph();
        let code_reports = code_analyzer.analyze_code(&graph).unwrap();
        println!("{:?}", code_reports);
        assert!(code_reports.len() > 0)
    }
}
