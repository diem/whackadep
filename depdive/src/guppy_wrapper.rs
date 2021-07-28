use guppy::graph::{PackageGraph, PackageMetadata};

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
