<template>
  <!-- information -->
  <section>
    <Information />

    <hr />

    <!-- statistics -->
    <Statistics :dependencies="$store.state.dependencies" />

    <hr />

    <!-- rustsec advisories -->
    <h2>RUSTSEC advisories without updates</h2>
    <div class="alert alert-info">
      These are dependencies that have RUST advisories associated to them, but
      no updates available to "fix" the advisory. Usually, the advisory comes
      with a recommendation on what crate can be used in place of the current
      one.
    </div>
    <RustsecTable
      :repo="repo"
      :dependencies="$store.getters.rustsec_no_updates"
    />

    <hr />

    <h2>Updates available for non-dev dependencies</h2>
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
    <DependenciesTable
      :repo="repo"
      :dependencies="$store.getters.non_dev_updatable_deps"
    />

    <hr />

    <h2>Updates available for dev dependencies</h2>
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
    <DependenciesTable
      :repo="repo"
      :dependencies="$store.getters.dev_updatable_deps"
    />

    <hr />

    <h2>Updates that can't be applied for dependencies</h2>
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
    <DependenciesTable
      :repo="repo"
      :dependencies="$store.getters.cant_update_deps"
    />
  </section>
</template>

<script>
import DependenciesTable from "./Dependencies.vue";
import Information from "./Information.vue";
import Statistics from "./Statistics.vue";
import RustsecTable from "./Rustsec.vue";

export default {
  name: "Dashboard",
  props: {
    repo: String,
  },
  components: {
    DependenciesTable,
    Information,
    RustsecTable,
    Statistics,
  },
};
</script>


<style scoped>
.header {
  position: sticky;
  top: 0;
}
</style>
