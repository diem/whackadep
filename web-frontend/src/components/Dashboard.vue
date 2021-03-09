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
  data() {
    return {
      // analysis to display
      commit: "",
      date: "",
      change_summary: null,
      dependencies: [],
      dev_updatable_deps: [],
      non_dev_updatable_deps: [],
      cant_update_deps: [],

      rustsec: [],
      rustsec_no_updates: [],
    };
  },
  props: {
    repo: String,
  },
  created() {
    console.log(`dashboard mounted with repo: ${this.repo}`);
  },
  components: {
    DependenciesTable,
    Information,
    RustsecTable,
    Statistics,
  },
  methods: {
    // reset data
    reset_data() {
      this.commit = "";
      this.date = "";
      this.change_summary = null;
      this.dependencies = [];
      this.dev_updatable_deps = [];
      this.non_dev_updatable_deps = [];
      this.cant_update_deps = [];

      this.rustsec = [];
      this.rustsec_no_updates = [];
    },
    // create a toast (a notification on the top right of the screen)
    toast(title, msg, variant = null) {
      this.$bvToast.toast(msg, {
        title: title,
        autoHideDelay: 5000,
        appendToast: true,
        variant: variant,
        solid: true,
      });
    },
  },
};
</script>


<style scoped>
.header {
  position: sticky;
  top: 0;
}
</style>
