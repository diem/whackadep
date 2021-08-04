use anyhow::Result;
use guppy::graph::{DependencyDirection, PackageGraph, PackageMetadata};
use guppy::PackageId;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DependencyKind {
    Normal,
    Build,
    Dev,
}

pub(crate) fn get_direct_dependencies(graph: &PackageGraph) -> Vec<PackageMetadata> {
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

pub(crate) fn get_all_dependencies(graph: &PackageGraph) -> Vec<PackageMetadata> {
    graph
        .query_workspace()
        .resolve_with_fn(|_, link| !link.to().in_workspace())
        .packages(guppy::graph::DependencyDirection::Forward)
        .filter(|pkg| !pkg.in_workspace())
        .collect()
}

pub(crate) fn get_package_dependencies<'a>(
    graph: &'a PackageGraph,
    package: &PackageMetadata,
) -> Result<Vec<PackageMetadata<'a>>> {
    let dependencies: Vec<PackageMetadata> = graph
        .query_forward(iter::once(package.id()))?
        .resolve()
        .packages(DependencyDirection::Forward)
        .filter(|pkg| pkg.id() != package.id())
        .collect();
    Ok(dependencies)
}

pub(crate) fn filter_exclusive_deps<'a>(
    package: &'a PackageMetadata,
    pacakge_dependencies: &[PackageMetadata<'a>],
) -> Vec<PackageMetadata<'a>> {
    // HashSet for quick lookup in dependency subtree
    let mut package_deps: HashSet<&PackageId> =
        pacakge_dependencies.iter().map(|dep| dep.id()).collect();
    // Add root to the tree
    package_deps.insert(package.id());

    // Keep track of non-exclusive deps
    let mut common_deps: HashSet<&PackageId> = HashSet::new();
    // and exclusive ones for
    let mut exclusive_deps: HashMap<&PackageId, PackageMetadata> = HashMap::new();

    for dep in pacakge_dependencies {
        let mut unique = true;
        for link in dep.reverse_direct_links() {
            let from_id = link.from().id();
            if !package_deps.contains(from_id) || common_deps.contains(from_id) {
                unique = false;
                common_deps.insert(dep.id());
                break;
            }
        }
        if unique {
            exclusive_deps.insert(dep.id(), *dep);
        }
    }

    exclusive_deps.values().cloned().collect()
}

/// This function takes a guppy graph
/// and returns a map indicating the dependency kind
/// for all `crate:version` in the dep graph
pub(crate) fn get_dep_kind_map(
    graph: &PackageGraph,
) -> Result<HashMap<(String, Version), DependencyKind>> {
    let mut hm: HashMap<(String, Version), DependencyKind> = HashMap::new();

    let normal_deps = get_normal_dependencies(graph);
    normal_deps.iter().for_each(|dep| {
        hm.insert(
            (dep.name().to_string(), dep.version().clone()),
            DependencyKind::Normal,
        );
    });

    // Get build deps
    // 1. Query graph for normal and build deps
    // (to ensure build deps of a normal dep get included)
    // 2. Filter out normal deps
    let build_deps: Vec<PackageMetadata> = graph
        .query_workspace()
        .resolve_with_fn(|_, link| {
            !link.to().in_workspace() && (link.build().is_present() || link.normal().is_present())
        })
        .packages(guppy::graph::DependencyDirection::Forward)
        .filter(|pkg| {
            !pkg.in_workspace()
                && !hm.contains_key(&(pkg.name().to_string(), pkg.version().clone()))
        })
        .collect();
    build_deps.iter().for_each(|dep| {
        hm.insert(
            (dep.name().to_string(), dep.version().clone()),
            DependencyKind::Build,
        );
    });

    // Get dev deps
    // 1. get direct dev deps of the graph
    // 2. get all the deps of the dev deps
    // 3. Filter out normal and build deps
    let direct_dev_deps: Vec<PackageMetadata> = graph
        .query_workspace()
        .resolve_with_fn(|_, link| !link.to().in_workspace() && link.dev().is_present())
        .packages(guppy::graph::DependencyDirection::Forward)
        .filter(|pkg| {
            !pkg.in_workspace()
                && !hm.contains_key(&(pkg.name().to_string(), pkg.version().clone()))
        })
        .collect();
    direct_dev_deps.iter().for_each(|dep| {
        hm.insert(
            (dep.name().to_string(), dep.version().clone()),
            DependencyKind::Dev,
        );
    });
    for dep in &direct_dev_deps {
        let indirect_dev_deps: Vec<PackageMetadata> = get_package_dependencies(graph, dep)?
            .into_iter()
            .filter(|pkg| {
                !pkg.in_workspace()
                    && !hm.contains_key(&(pkg.name().to_string(), pkg.version().clone()))
            })
            .collect();
        indirect_dev_deps.iter().for_each(|dep| {
            hm.insert(
                (dep.name().to_string(), dep.version().clone()),
                DependencyKind::Dev,
            );
        });
    }

    Ok(hm)
}

pub(crate) fn get_normal_dependencies(graph: &PackageGraph) -> Vec<PackageMetadata> {
    graph
        .query_workspace()
        .resolve_with_fn(|_, link| !link.to().in_workspace() && link.normal().is_present())
        .packages(guppy::graph::DependencyDirection::Forward)
        .filter(|pkg| !pkg.in_workspace())
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use guppy::CargoMetadata;

    #[test]
    fn test_guppy_wrapper_exclusive_deps() {
        let metadata = CargoMetadata::parse_json(include_str!(
            "../resources/test/exclusive_dep_cargo_metadata.json"
        ))
        .unwrap();
        let graph = metadata.build_graph().unwrap();
        let total_dependencies = get_all_dependencies(&graph).len();

        let package = graph.packages().find(|p| p.name() == "gitlab").unwrap();
        let dependencies = get_package_dependencies(&graph, &package).unwrap();
        let gitlab_exclusive_deps = filter_exclusive_deps(&package, &dependencies).len();
        let common_deps = dependencies.len() - gitlab_exclusive_deps;

        let package = graph.packages().find(|p| p.name() == "octocrab").unwrap();
        let dependencies = get_package_dependencies(&graph, &package).unwrap();
        let octocrab_exclusive_deps = filter_exclusive_deps(&package, &dependencies).len();

        assert_eq!(
            total_dependencies,
            common_deps + gitlab_exclusive_deps + octocrab_exclusive_deps + 2
        );
    }

    #[test]
    fn test_guppy_wrapper_dep_kind() {
        let metadata =
            CargoMetadata::parse_json(include_str!("../resources/test/depkind_metadata.json"))
                .unwrap();
        let graph = metadata.build_graph().unwrap();

        let hm = get_dep_kind_map(&graph).unwrap();

        assert_eq!(
            145,
            hm.values()
                .filter(|k| matches!(k, DependencyKind::Normal))
                .count()
        );
        assert_eq!(
            12,
            hm.values()
                .filter(|k| matches!(k, DependencyKind::Build))
                .count()
        );
        assert_eq!(
            16,
            hm.values()
                .filter(|k| matches!(k, DependencyKind::Dev))
                .count()
        );
    }
}
