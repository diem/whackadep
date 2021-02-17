<template>
  <!-- information -->
  <section>
    <Information />

    <hr />

    <!-- statistics -->
    <Statistics :dependencies="$store.state.dependencies" />

    <hr />

    <!-- rustsec advisories -->
    <h2>
      RUSTSEC advisories without updates ({{
        length($store.getters.rustsec_no_updates)
      }})
    </h2>
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

    <h2>
      Updates available for non-dev dependencies ({{
        length($store.getters.non_dev_updatable_deps)
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
    <DependenciesTable
      :repo="repo"
      :dependencies="$store.getters.non_dev_updatable_deps"
    />

    <hr />

    <h2>
      Updates available for dev dependencies ({{
        length($store.getters.dev_updatable_deps)
      }})
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
    <DependenciesTable
      :repo="repo"
      :dependencies="$store.getters.dev_updatable_deps"
    />

    <hr />

    <h2>
      Updates that can't be applied for dependencies ({{
        length($store.getters.cant_update_deps)
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
    <DependenciesTable
      :repo="repo"
      :dependencies="$store.getters.cant_update_deps"
    />
  </section>
</template>

<script>
import axios from "axios";

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
  mounted() {
    console.log(`dashboard mounted with repo: ${this.repo}`);
    this.get_dependencies();
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
    // obtains the latest analysis result for a repo
    get_dependencies() {
      axios
        .get("/dependencies?repo=" + this.repo)
        .then((response) => {
          //
          // Error handling
          //

          // TODO: return an error code from the server instead?
          if (response.data.constructor == String) {
            this.toast("Information", response.data, "info");
            this.reset_data();
            return;
          }

          //
          // Retrieving data
          //

          // TODO: this vuex store will replace everything here
          this.$store.dispatch("add_analysis", response.data);

          //
          // alert on vuln
          //

          if (
            response.data.rust_dependencies.rustsec.vulnerabilities.length > 0
          ) {
            this.toast(
              "RUSTSEC",
              `vulnerabilities found: ${this.rustsec.vulnerabilities
                .map((vuln) => vuln.advisory.id)
                .join(", ")}`,
              "danger"
            );
          }

          // notification
          this.toast(
            "Retrieving analysis",
            `latest analysis successfuly retrieved for ${this.repo}`,
            "success"
          );
        })
        .catch((error) => {
          console.log(error);
          if (error.response) {
            // The request was made and the server responded with a status code
            // that falls out of the range of 2xx
            this.toast("Error from the server", error.message, "danger");
          } else if (error.request) {
            // The request was made but no response was received
            // `error.request` is an instance of XMLHttpRequest in the browser and an instance of
            // http.ClientRequest in node.js
            this.toast(
              "server unavailable",
              `more information: ${JSON.stringify(error.message)}`,
              "danger"
            );
          } else {
            // Something happened in setting up the request that triggered an Error
            this.toast(
              "unknown error",
              `more information: ${JSON.stringify(error.message)}`,
              "danger"
            );
          }
          console.log(error.config);
        });
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
    // length or 0 if undefined
    length(thing) {
      if (typeof thing == Array) {
        return thing.length;
      } else {
        return 0;
      }
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
