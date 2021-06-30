use anyhow::Result;
use depdive::{
    DependencyGraphAnalyzer, TabledCrateSourceDiffReport, TabledCratesioReport, TabledGitHubReport,
};
use guppy::MetadataCommand;
use tabled::{Style, Table};

fn main() -> Result<()> {
    let graph = MetadataCommand::new().build_graph()?;

    let depdive_report = DependencyGraphAnalyzer.analyze_dep_graph(&graph)?;

    let table: Vec<TabledCratesioReport> = depdive_report
        .crate_stats
        .iter()
        .map(|r| r.tabled_cratesio_report.clone())
        .collect();
    let table = Table::new(table).with(Style::github_markdown()).to_string();
    println!("{}", table);

    let table: Vec<TabledGitHubReport> = depdive_report
        .crate_stats
        .iter()
        .map(|r| r.tabled_github_report.clone())
        .collect();
    let table = Table::new(table).with(Style::github_markdown()).to_string();
    println!("{}", table);

    let table: Vec<TabledCrateSourceDiffReport> = depdive_report
        .crate_stats
        .iter()
        .map(|r| r.tabled_crate_source_diff_report.clone())
        .collect();
    let table = Table::new(table).with(Style::github_markdown()).to_string();
    println!("{}", table);

    let table = Table::new(depdive_report.code_stats)
        .with(Style::github_markdown())
        .to_string();
    println!("{}", table);

    Ok(())
}
