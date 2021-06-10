//! This crate contains a number of analyses that can be run on dependencies

use guppy::graph::PackageMetadata;
use tabled::Tabled;

#[derive(Tabled)]
pub struct DependencyReport<'a> {
    pub name: &'a str,
    pub has_build_script: bool,
}

pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    pub fn analyze_dep<'a>(&self, package: &PackageMetadata<'a>) -> DependencyReport<'a> {
        let name = package.name();
        let has_build_script = package.has_build_script();

        // Add more analyses here

        DependencyReport {
            name,
            has_build_script,
        }
    }
}
