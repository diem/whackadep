use anyhow::{ensure, Result};
use cargo_audit::{
    auditor::Auditor,
    config::{AuditConfig, OutputFormat},
};
use rustsec::Report;
use std::path::Path;
use tokio::process::Command;

// TODO: what if there is no lock file? This will crash
// TODO: should we generate the lock file ourselves first? (cargo-generate-lockfile)
pub async fn rustsec(repo_path: &Path) -> Result<Report> {
    // make sure a Carg.lock file is there
    generate_lockfile(repo_path).await?;

    // run audit
    let mut audit_config = AuditConfig::default();
    audit_config.output.format = OutputFormat::Json;
    let mut auditor = Auditor::new(&audit_config);
    let report = auditor.audit(Some(&repo_path.join("Cargo.lock")));

    //
    return Ok(report);
}

pub async fn generate_lockfile(repo_path: &Path) -> Result<()> {
    let output = Command::new("cargo")
        .current_dir(repo_path)
        .arg("generate-lockfile")
        .output()
        .await?;

    ensure!(
        output.status.success(),
        "couldn't run cargo generate-lockfile: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}
