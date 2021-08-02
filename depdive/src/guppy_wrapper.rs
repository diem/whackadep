use anyhow::Result;
use guppy::graph::{DependencyDirection, PackageGraph, PackageMetadata};
use guppy::PackageId;
use std::collections::{HashMap, HashSet};
use std::iter;

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
}
