import semver from "semver";

export function calculate_priority_score(dep) {
  var priority_score = 0;
  var priority_reasons = [];

  // version change
  var type_of_change = version_change(dep);
  if (type_of_change == "major") {
    priority_score += 10;
    priority_reasons.push("MAJOR version change");
  } else if (type_of_change == "minor") {
    priority_score += 3;
    priority_reasons.push("MINOR version change");
  } else if (type_of_change == "patch") {
    priority_score += 1;
    priority_reasons.push("PATCH version change");
  }

  // RUSTSEC
  if (dep.vulnerabilities) {
    priority_score += 30;
    priority_reasons.push("RUSTSEC vulnerability associated");
  }

  if (dep.warnings) {
    priority_score += 20;
    priority_reasons.push("RUSTSEC warning associated");
  }

  //
  return { priority_score, priority_reasons };
}

export function version_change(dep) {
  var version = dep.version;
  var new_version = dep.update.versions[dep.update.versions.length - 1];
  // rust has the tendency to lie when

  var type_change = semver.diff(version, new_version);
  return type_change;
}

