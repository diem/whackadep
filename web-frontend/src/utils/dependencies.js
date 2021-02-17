import semver from "semver";

// This checks if a dependency can be updated in several senses:
// - if it's a direct dependency, can it be updated easily (no breaking changes, if the developers respected Rust variant of semver)
// - if it's a transitive dependency, can we update it at all?
// https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements
// > An update is allowed if the new version number does not modify the left-most non-zero digit in the major, minor, patch grouping
// > This compatibility convention is different from SemVer in the way it treats versions before 1.0.0. While SemVer says there is no compatibility before 1.0.0, Cargo considers 0.x.y to be compatible with 0.x.z, where y ≥ z and x > 0.
export function update_allowed(dependency) {
  var version = dependency.version;
  var new_version =
    dependency.update.versions[dependency.update.versions.length - 1];

  var pre = predicate(version);
  return semver.satisfies(new_version, pre);
}

function predicate(version) {
  var major = semver.major(version);
  if (major != 0) {
    return `${major}.x`;
  }
  var minor = semver.minor(version);
  if (minor != 0) {
    return `${major}.${minor}.x`;
  }
  var patch = semver.patch(version);
  if (patch != 0) {
    return `${major}.${minor}.${patch}.x`;
  }
  var prerelease = semver.prerelease(version);
  if (prerelease != 0) {
    return `${major}.${minor}.${patch}.${prerelease}.x`;
  }
  // if we can't figure it out, avoid false negative by
  // return a predicate that will say "yes we can update this"
  return "x";
}