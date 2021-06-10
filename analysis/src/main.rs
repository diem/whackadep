use analysis::{DependencyAnalyzer, DependencyReport};
use anyhow::Result;
use guppy::MetadataCommand;
use tabled::table;

fn main() -> Result<()> {
    let graph = MetadataCommand::new().build_graph()?;

    let direct_dependencies: Vec<_> = graph
        .query_workspace()
        .resolve_with_fn(|_, link| {
            // Collect direct dependencies of workspace packages.
            let (from, to) = link.endpoints();
            from.in_workspace() && !to.in_workspace()
        })
        .packages(guppy::graph::DependencyDirection::Forward)
        .filter(|pkg| !pkg.in_workspace())
        .collect();

    // Run Analysis on each direct dependency
    let reports: Vec<DependencyReport> = direct_dependencies
        .iter()
        .map(|pkg| DependencyAnalyzer.analyze_dep(pkg))
        .collect();

    let table = table!(&reports);
    println!("{}", table);

    Ok(())
}
