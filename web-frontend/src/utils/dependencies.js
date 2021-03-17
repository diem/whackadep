import semver from "semver";

import { calculate_priority_score } from "@/engines/priority";
import { calculate_risk_score } from "@/engines/risk";

// This checks if a dependency can be updated in several senses:
// - if it's a direct dependency, can it be updated easily (no breaking changes, if the developers respected Rust variant of semver)
// - if it's a transitive dependency, can we update it at all?
// https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements
// > An update is allowed if the new version number does not modify the left-most non-zero digit in the major, minor, patch grouping
// > This compatibility convention is different from SemVer in the way it treats versions before 1.0.0. While SemVer says there is no compatibility before 1.0.0, Cargo considers 0.x.y to be compatible with 0.x.z, where y ≥ z and x > 0.

export function update_allowed(dependency) {
  let version = dependency.version;
  let new_version =
    dependency.update.versions[dependency.update.versions.length - 1];

  let pre = predicate(version);
  return semver.satisfies(new_version, pre);
}

function predicate(version) {
  let major = semver.major(version);
  if (major != 0) {
    return `${major}.x`;
  }
  let minor = semver.minor(version);
  if (minor != 0) {
    return `${major}.${minor}.x`;
  }
  let patch = semver.patch(version);
  if (patch != 0) {
    return `${major}.${minor}.${patch}.x`;
  }
  let prerelease = semver.prerelease(version);
  if (prerelease != 0) {
    return `${major}.${minor}.${patch}.${prerelease}.x`;
  }
  // if we can't figure it out, avoid false negative by
  // return a predicate that will say "yes we can update this"
  return "x";
}


//
// Transform the analysis (adds new fields to all dependencies)
//

export function sort_priority(a, b) {
  return a.priority_score > b.priority_score ? -1 : 1;
}

export function transform_analysis(dependencies, rustsec) {

  dependencies.forEach((dependency) => {

    // add rustsec vulnerabilities to the relevant dependencies
    rustsec.vulnerabilities.forEach((vuln) => {
      if (vuln.package.name == dependency.name) {
        let patched = vuln.versions.patched;
        let unaffected = vuln.versions.unaffected;
        let affected =
          !semver.satisfies(dependency.version, patched) &&
          !semver.satisfies(dependency.version, unaffected);
        if (affected) {
          if (Array.isArray(dependency["vulnerabilities"])) {
            dependency.vulnerabilities.push(vuln);
          } else {
            dependency.vulnerabilities = [vuln];
          }
        }
      }
    });

    // add rustsec warnings to the relevant dependencies
    for (const warnings of Object.values(rustsec.warnings)) {
      warnings.forEach((warning) => {
        if (warning.package.name == dependency.name) {
          if (Array.isArray(dependency["warnings"])) {
            dependency.warnings.push(warning);
          } else {
            dependency.warnings = [warning];
          }
        }
      });
    }

    // only modify dependencies that have update now
    if (dependency.update != null) {
      // can we update this?
      if (dependency.direct || update_allowed(dependency)) {
        dependency.update_allowed = true;
      } else {
        dependency.update_allowed = false;
      }

      // priority score
      let {
        priority_score,
        priority_reasons,
      } = calculate_priority_score(dependency);
      dependency.priority_score = priority_score;
      dependency.priority_reasons = priority_reasons;

      // risk score
      let { risk_score, risk_reasons } = calculate_risk_score(
        dependency
      );
      dependency.risk_score = risk_score;
      dependency.risk_reasons = risk_reasons;
    }

    // end of adding new fields to all dependencies
  });
}

