<template>
  <div class="container">
    <!-- header/nav -->
    <nav class="navbar navbar-expand-lg navbar-dark bg-dark">
      <span class="navbar-brand mb-0 h1">Whack-a-dep!</span>
      <button
        class="navbar-toggler"
        type="button"
        data-toggle="collapse"
        data-target="#navbarText"
        aria-controls="navbarText"
        aria-expanded="false"
        aria-label="Toggle navigation"
      >
        <span class="navbar-toggler-icon"></span>
      </button>
      <div class="collapse navbar-collapse" id="navbarText">
        <ul class="navbar-nav mr-auto">
          <li class="nav-item active">
            <a class="nav-link" href="/">
              Home <span class="sr-only">(current)</span>
            </a>
          </li>
          <li class="nav-item">
            <a class="nav-link" @click.prevent="refresh" href="#">Refresh</a>
          </li>
        </ul>
      </div>

      <b-dropdown id="dropdownrepo" :text="current_repo" class="m-md-2">
        <b-dropdown-item
          v-for="repo in repos"
          v-bind:key="repo"
          :disabled="repo == current_repo"
          @click="switch_repo(repo)"
        >
          {{ repo }}
        </b-dropdown-item>
        <b-dropdown-divider></b-dropdown-divider>
        <b-dropdown-item @click="showModal"
          >add a new rust repository</b-dropdown-item
        >
      </b-dropdown>
    </nav>

    <!-- information -->
    <section>
      <section class="alert alert-warning">
        <h2 class="alert-heading">Information</h2>
        date: {{ date }}
        <br />
        commit:
        <a
          :href="'https://github.com/diem/diem/commit/' + commit"
          target="_blank"
          ><code>{{ commit }}</code></a
        ><br />
        <div v-if="change_summary">
          <div v-if="change_summary.new_updates.length > 0">
            <hr />
            new updates:
            <ul>
              <li
                v-for="d in change_summary.new_updates"
                v-bind:key="d.name + d.version + d.direct + d.dev"
              >
                [{{ d.direct ? "direct" : "transitive" }}]
                <a :href="'#' + d.name + d.version + d.direct + d.dev">{{
                  d.name
                }}</a>
                (<small
                  >{{ d.version }} → {{ d.update.versions.join(" → ") }}</small
                >)
              </li>
            </ul>
          </div>

          <div v-if="change_summary.new_rustsec.length > 0">
            <hr />
            new RUSTSEC advisories:
            <li
              v-for="r in change_summary.new_rustsec"
              v-bind:key="r.advisory.id"
            >
              <strong>{{ r.advisory.id }}</strong> - {{ r.advisory.title }}
            </li>
          </div>
        </div>
      </section>

      <hr />

      <!-- statistics -->
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

      <hr />

      <!-- rustsec advisories -->
      <h2>RUSTSEC advisories without updates</h2>
      <div class="alert alert-info">
        These are dependencies that have RUST advisories associated to them, but
        no updates available to "fix" the advisory. Usually, the advisory comes
        with a recommendation on what crate can be used in place of the current
        one.
      </div>
      <RustsecTable v-bind:dependencies="rustsec_no_updates" />

      <hr />

      <h2>
        Updates available for non-dev dependencies ({{
          count(non_dev_updatable_deps)
        }})
      </h2>
      <div class="alert alert-info">
        These are non-dev dependencies that can be updated either because they
        are direct dependencies or because they are transitive and do not have
        breaking changes (according to
        <a
          href="https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements"
          >Rust semantic</a
        >
        about semver).
      </div>
      <DependenciesTable v-bind:dependencies="non_dev_updatable_deps" />

      <hr />

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

      <hr />

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
        modify the left-most non-zero digit in the major, minor, patch
        grouping").
      </div>
      <DependenciesTable v-bind:dependencies="cant_update_deps" />
    </section>
    <!-- /container -->

    <!-- modal -->
    <b-modal ref="modal" hide-footer title="Adding a new repo">
      <div>
        <b-form @submit="onSubmit" @reset="onReset">
          <b-form-group
            id="repo-group"
            label="Git repository:"
            label-for="input-1"
            description="We only support Rust repositories at the moment."
          >
            <b-form-input
              id="repo"
              v-model="form.repo"
              placeholder="Enter git repository"
              required
            ></b-form-input>
          </b-form-group>
          <b-button type="submit" variant="primary">Add</b-button>
        </b-form>
      </div>
    </b-modal>
  </div>
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

function calculate_risk_score(dep) {
  var risk_score = 0;
  var risk_reasons = [];

  if (dep.update.build_rs) {
    risk_score += 10;
    risk_reasons.push("<code>build.rs</code> file Changed");
  }

  return { risk_score, risk_reasons };
}

function sort_priority(a, b) {
  return a.priority_score > b.priority_score ? -1 : 1;
}

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
      all_rustsec: [],
      rustsec_no_updates: [],

      // repo mgmt
      current_repo: "https://github.com/diem/diem.git",
      repos: [],
      form: {
        repo: "",
      },
    };
  },
  mounted() {
    this.get_repos();
    this.get_dependencies();
  },
  components: {
    DependenciesTable,
    RustsecTable,
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
      this.all_rustsec = [];
      this.rustsec_no_updates = [];
    },
    onSubmit(event) {
      event.preventDefault();
      this.hideModal();
      axios
        .post("/add_repo", this.form)
        .then((response) => {
          // TODO: return an error code from the server instead?
          if (response.data == "ok") {
            this.toast("Git repository added", "success", "success");
            this.get_repos();
          } else {
            this.toast(
              "Problem adding git repository",
              response.data,
              "danger"
            );
          }
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
      this.onReset();
    },
    onReset() {
      // Reset our form values
      this.form.repo = "";
    },
    create_repo() {
      this.showModal();
    },
    showModal() {
      this.$refs["modal"].show();
    },
    hideModal() {
      this.$refs["modal"].hide();
      this.modal_text = "";
    },
    switch_repo(repo) {
      console.log(repo);
      this.current_repo = repo;
      //      this.$refs.dropdownrepo.text(repo);
      this.get_dependencies();
    },
    get_repos() {
      axios
        .get("/repos")
        .then((response) => {
          this.repos = response.data;
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
    // obtains the latest analysis result for a repo
    get_dependencies() {
      axios
        .get("/dependencies?repo=" + this.current_repo)
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

          // retrieve commit
          this.commit = response.data.commit;

          // retrieve datetime
          this.date = new Date(response.data.timestamp);

          // retrieve change summary
          this.change_summary = response.data.rust_dependencies.change_summary;

          // retrieve all rust dependencies
          this.dependencies = response.data.rust_dependencies.dependencies;

          // retrieve rustsec
          this.rustsec = response.data.rust_dependencies.rustsec;

          //
          // Transforming data
          //

          // collect all the rustsec in one table
          if (this.rustsec.vulnerabilities.length > 0) {
            this.toast(
              "RUSTSEC",
              `vulnerabilities found: ${this.rustsec.vulnerabilities
                .map((vuln) => vuln.advisory.id)
                .join(", ")}`,
              "danger"
            );
          }
          this.all_rustsec = [...this.rustsec.vulnerabilities];
          for (const warnings of Object.values(this.rustsec.warnings)) {
            this.all_rustsec = this.all_rustsec.concat(warnings);
          }

          // add new fields to all dependencies
          this.dependencies.forEach((dependency) => {
            // add rustsec vulnerabilities to the relevant dependencies
            this.rustsec.vulnerabilities.forEach((vuln) => {
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
            for (const warnings of Object.values(this.rustsec.warnings)) {
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
              if (dependency.direct || this.update_allowed(dependency)) {
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

          //
          // Filter
          //

          var updatable_dependencies = this.dependencies.filter(
            (dependency) => dependency.update != null
          );

          // filter for dependencies that have a RUSTSEC advisory but can't be updated
          this.rustsec_no_updates = this.dependencies.filter((dependency) => {
            return (
              (dependency.vulnerabilities != null ||
                dependency.warnings != null) &&
              dependency.update == null
            );
          });

          // filter for dependencies that have updates
          var can_update_dependencies = updatable_dependencies.filter(
            (dependency) => dependency.update_allowed
          );

          // filter for non-dev dependencies that have an update
          this.non_dev_updatable_deps = can_update_dependencies.filter(
            (dependency) => !dependency.dev
          );
          this.non_dev_updatable_deps = this.non_dev_updatable_deps.sort(
            sort_priority
          );

          // filter for dev dependencies that have an update
          this.dev_updatable_deps = can_update_dependencies.filter(
            (dependency) => dependency.dev
          );
          this.dev_updatable_deps = this.dev_updatable_deps.sort(sort_priority);

          // finally, retrieve dependencies that have updates and _can't_ be updated
          this.cant_update_deps = updatable_dependencies.filter(
            (dependency) => !dependency.update_allowed
          );
          this.cant_update_deps = this.cant_update_deps.sort(sort_priority);

          // notification
          this.toast(
            "Retrieving analysis",
            `latest analysis successfuly retrieved for ${this.current_repo}`,
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
    refresh() {
      axios.get("/refresh?repo=" + this.current_repo).then((response) => {
        this.toast("Refresh requested", response.data, "info");
      });
    },
    toast(title, msg, variant = null) {
      this.$bvToast.toast(msg, {
        title: title,
        autoHideDelay: 5000,
        appendToast: true,
        variant: variant,
        solid: true,
      });
    },
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
nav {
  margin-bottom: 20px;
}

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
