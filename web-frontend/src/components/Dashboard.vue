<template>
  <section>
    <div class="alert alert-warning">
      commit: <code>{{ commit }}</code>
    </div>

    <div class="row" id="stats">
      <div class="col-sm bg-light bg-gradient p-5">
        <strong>{{ direct_dependencies }} </strong>
        <small> non-dev direct dependencies</small>
      </div>
      <div class="col-sm p-5 bg-light bg-gradient">
        <strong>{{ transitive_dependencies }} </strong>
        <small> non-dev transitive dependencies</small>
      </div>
      <div class="col-sm bg-light bg-gradient p-5">
        <strong>{{ dev_dependencies }} </strong>
        <small> direct dev dependencies</small>
      </div>
    </div>

    <h2>RUSTSEC advisories without updates</h2>
    <div class="alert alert-info">
      These are dependencies that have RUST advisories associated to them, but
      no updates available to "fix" the advisory. Usually, the advisory comes
      with a recommendation on what crate can be used in place of the current
      one.
    </div>
    <RustsecTable v-bind:dependencies="rustsec" />

    <h2>
      Updates available for non-dev dependencies ({{
        count(non_dev_updatable_deps)
      }})
    </h2>
    <div class="alert alert-info">
      These are non-dev dependencies that can be updated either because they are
      direct dependencies or because they are transitive and do not have
      breaking changes (according to
      <a
        href="https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements"
        >Rust semantic</a
      >
      about semver).
    </div>
    <DependenciesTable v-bind:dependencies="non_dev_updatable_deps" />

    <h2>
      Updates available for dev dependencies ({{ count(dev_updatable_deps) }})
    </h2>
    <div class="alert alert-info">
      These are dev dependencies that can be updated either because they are
      direct dependencies or because they are transitive and do not have
      breaking changes (according to
      <a
        href="https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements"
        >Rust semantic</a
      >
      about semver).
    </div>
    <DependenciesTable v-bind:dependencies="dev_updatable_deps" />

    <h2>
      Updates that can't be applied for dependencies ({{
        count(cant_update_deps)
      }})
    </h2>
    <div class="alert alert-info">
      These are dependencies that have an update, but can't be updated because
      they are transitive dependencies and don't respect
      <a
        href="https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements"
        >Rust semantic</a
      >
      about semver ("An update is allowed if the new version number does not
      modify the left-most non-zero digit in the major, minor, patch grouping").
    </div>
    <DependenciesTable v-bind:dependencies="cant_update_deps" />
  </section>
</template>

<script>
import semver from "semver";
import axios from "axios";

import DependenciesTable from "./Dependencies.vue";
import RustsecTable from "./Rustsec.vue";

function version_change(dep) {
  var version = dep.version;
  var new_version = dep.update.versions[dep.update.versions.length - 1];
  // rust has the tendency to lie when

  var type_change = semver.diff(version, new_version);
  return type_change;
}

function calculate_priority_score(dep) {
  var priority_score = 0;
  var priority_reasons = [];

  // version change
  var type_of_change = version_change(dep);
  if (type_of_change == "major") {
    priority_score += 10;
    priority_reasons.push("MAJOR version change");
  } else if (type_of_change == "minor") {
    priority_score += 2;
    priority_reasons.push("MINOR version change");
  }

  // RUSTSEC
  if (dep.rustsec) {
    priority_score += 20;
    priority_reasons.push("RUSTSEC associated");
  }

  //
  return { priority_score, priority_reasons };
}

function sort_priority(a, b) {
  return a.priority_score > b.priority_score ? -1 : 1;
}

export default {
  name: "Dashboard",
  data() {
    return {
      commit: "",
      dependencies: [],
      dev_updatable_deps: [],
      non_dev_updatable_deps: [],
      cant_update_deps: [],
      rustsec: [],
    };
  },
  mounted() {
    axios.get("/dependencies").then((response) => {
      // retrieve commit
      this.commit = response.data.commit;

      // retrieve all rust dependencies
      this.dependencies = response.data.rust_dependencies.dependencies;
      console.log("all deps", this.dependencies);

      // filter for dependencies that have a RUSTSEC but no updates
      this.rustsec = response.data.rust_dependencies.dependencies.filter(
        (dependency) => dependency.rustsec != null && dependency.update == null
      );

      // filter for dependencies that have updates
      var updatable_dependencies = response.data.rust_dependencies.dependencies
        .filter((dependency) => dependency.update != null)
        .map((dependency) => {
          let { priority_score, priority_reasons } = calculate_priority_score(
            dependency
          );
          dependency.priority_score = priority_score;
          dependency.priority_reasons = priority_reasons;
          return dependency;
        });

      var can_update_dependencies = updatable_dependencies.filter(
        (dependency) => {
          if (!dependency.direct) {
            return this.update_allowed(dependency);
          } else {
            return true;
          }
        }
      );

      // filter for non-dev dependencies that have an update
      this.non_dev_updatable_deps = can_update_dependencies.filter(
        (dependency) => !dependency.dev
      );
      this.non_dev_updatable_deps = this.non_dev_updatable_deps.sort(
        sort_priority
      );
      console.log("non-dev update deps", this.non_dev_updatable_deps);

      // filter for dev dependencies that have an update
      this.dev_updatable_deps = can_update_dependencies.filter(
        (dependency) => dependency.dev
      );
      this.dev_updatable_deps = this.dev_updatable_deps.sort(sort_priority);

      // finally, retrieve dependencies that have updates and _can't_ be updated
      // just in case we made a mistake above...
      this.cant_update_deps = updatable_dependencies.filter((dependency) => {
        if (!dependency.direct) {
          return !this.update_allowed(dependency);
        } else {
          return false;
        }
      });
      this.cant_update_deps = this.cant_update_deps.sort(sort_priority);
    });
  },
  components: {
    DependenciesTable,
    RustsecTable,
  },
  methods: {
    count(deps) {
      if (deps != null) {
        // there will be redundant dependencies
        return deps.length;
      }
      return 0;
    },
    // This checks if a dependency can be updated in several senses:
    // - if it's a direct dependency, can it be updated easily (no breaking changes, if the developers respected Rust variant of semver)
    // - if it's a transitive dependency, can we update it at all?
    // https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements
    // > An update is allowed if the new version number does not modify the left-most non-zero digit in the major, minor, patch grouping
    // > This compatibility convention is different from SemVer in the way it treats versions before 1.0.0. While SemVer says there is no compatibility before 1.0.0, Cargo considers 0.x.y to be compatible with 0.x.z, where y ≥ z and x > 0.
    update_allowed(dependency) {
      var version = dependency.version;
      var new_version =
        dependency.update.versions[dependency.update.versions.length - 1];

      var pre = this.predicate(version);
      return semver.satisfies(new_version, pre);
    },
    predicate(version) {
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
    },
  },
  computed: {
    direct_dependencies() {
      if (this.dependencies != null) {
        // there will be redundant dependencies
        return this.dependencies.filter((dep) => !dep.dev && dep.direct).length;
      }
      return 0;
    },
    transitive_dependencies() {
      if (this.dependencies != null) {
        // there will be redundant dependencies
        return this.dependencies.filter((dep) => !dep.dev && !dep.direct)
          .length;
      }
      return 0;
    },
    dev_dependencies() {
      if (this.dependencies != null) {
        // there will be redundant dependencies
        return this.dependencies.filter((dep) => dep.dev && dep.direct).length;
      }
      return 0;
    },
  },
};
</script>

<style scoped>
#stats {
  text-align: center;
  margin-bottom: 20px;
}
#stats div {
  margin: 0 5px;
}

.header {
  position: sticky;
  top: 0;
}
</style>