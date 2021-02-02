<template>
  <section>
    <p>
      commit: <code>{{ commit }}</code>
    </p>

    <div class="row" id="stats">
      <div class="col-sm bg-warning bg-gradient p-5 center">
        <strong>{{ direct_dependencies }} </strong>
        <small> non-dev direct dependencies</small>
      </div>
      <div class="col-sm p-5 bg-info bg-gradient">
        <strong>{{ transitive_dependencies }} </strong>
        <small> non-dev transitive dependencies</small>
      </div>
      <div class="col-sm bg-success bg-gradient p-5">
        <strong>{{ dev_dependencies }} </strong>
        <small> direct dev dependencies</small>
      </div>
    </div>

    <h2>RUSTSEC advisories without updates</h2>
    <p>
      These are dependencies that have RUST advisories associated to them, but
      no updates available to "fix" the advisory. Usually, the advisory comes
      with a recommendation on what crate can be used in place of the current
      one.
    </p>
    <DependenciesTable v-bind:dependencies="rustsec" />

    <h2>
      Updates available for non-dev dependencies ({{
        count(non_dev_updatable_deps)
      }})
    </h2>
    <p>
      These are non-dev dependencies that can be updated either because they are
      direct dependencies or because they are transitive and do not have
      breaking changes (according to Rust semantic about semver).
    </p>
    <DependenciesTable v-bind:dependencies="non_dev_updatable_deps" />

    <h2>
      Updates available for dev dependencies ({{ count(dev_updatable_deps) }})
    </h2>
    <p>
      These are dev dependencies that can be updated either because they are
      direct dependencies or because they are transitive and do not have
      breaking changes (according to Rust semantic about semver).
    </p>
    <DependenciesTable v-bind:dependencies="dev_updatable_deps" />

    <h2>
      Updates that can't be applied for dependencies ({{
        count(cant_update_deps)
      }})
    </h2>
    <p>
      These are dependencies that have an update, but can't be updated because
      they are transitive dependencies and don't respect Rust semantic about
      semver ("An update is allowed if the new version number does not modify
      the left-most non-zero digit in the major, minor, patch grouping").
    </p>
    <DependenciesTable v-bind:dependencies="cant_update_deps" />
  </section>
</template>

<script>
import DependenciesTable from "./Dependencies.vue";

export default {
  name: "Dashboard",
  data() {
    return {
      commit: null,
      dependencies: null,
      dev_updatable_deps: null,
      non_dev_updatable_deps: null,
      rustsec: null,
    };
  },
  inject: {
    axios: {
      from: "axios",
    },
    semver: {
      from: "semver",
    },
  },
  mounted() {
    this.axios.get("/dependencies").then((response) => {
      // retrieve commit
      this.commit = response.data.commit;

      // retrieve all rust dependencies
      this.dependencies = response.data.rust_dependencies.dependencies;
      console.log(response.data.rust_dependencies.dependencies);

      // filter for dependencies that have a RUSTSEC but no updates
      this.rustsec = response.data.rust_dependencies.dependencies.filter(
        (dependency) => dependency.rustsec != null && dependency.update == null
      );

      // filter for dependencies that have updates and _can_ be updated
      var updatable_dependencies = response.data.rust_dependencies.dependencies
        .filter((dependency) => dependency.update != null)
        .filter((dependency) => {
          if (!dependency.direct) {
            return this.update_allowed(dependency);
          } else {
            return true;
          }
        });

      // filter for non-dev dependencies that have an update
      this.non_dev_updatable_deps = updatable_dependencies.filter(
        (dependency) => !dependency.dev
      );

      // filter for dev dependencies that have an update
      this.dev_updatable_deps = updatable_dependencies.filter(
        (dependency) => dependency.dev
      );

      // finally, retrieve dependencies that have updates and _can't_ be updated
      // just in case we made a mistake above...
      this.cant_update_deps = response.data.rust_dependencies.dependencies
        .filter((dependency) => dependency.update != null)
        .filter((dependency) => {
          if (!dependency.direct) {
            return !this.update_allowed(dependency);
          } else {
            return false;
          }
        });
    });
  },
  components: {
    DependenciesTable,
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
      return this.semver.satisfies(new_version, pre);
    },
    predicate(version) {
      var major = this.semver.major(version);
      if (major != 0) {
        return `${major}.x`;
      }
      var minor = this.semver.minor(version);
      if (minor != 0) {
        return `${major}.${minor}.x`;
      }
      var patch = this.semver.patch(version);
      if (patch != 0) {
        return `${major}.${minor}.${patch}.x`;
      }
      var prerelease = this.semver.prerelease(version);
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