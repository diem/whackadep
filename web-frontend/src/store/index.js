import Vue from "vue";
import Vuex from "vuex";

import axios from "axios";

import { transform_analysis, sort_priority } from "@/utils/dependencies";

Vue.use(Vuex);

//
// Initial state
//

const getDefaultState = () => {
  return {
    repo: "",
    commit: "",
    date: "",
    change_summary: {
      new_updates: [],
      new_rustsec: {
        vulnerabilities: [],
        warnings: [],
      }
    },
    dependencies: [],
    dependency_map: {},
    rustsec: [],
  }
}

const state = getDefaultState()

//
// Store declaration
//

export default new Vuex.Store({
  state,
  getters: {
    // dependencies that have an update available
    updatable_dependencies: state => {
      return state.dependencies.filter((dependency) => dependency.update != null);
    },
    // dependencies that have a RUSTSEC advisory but can't be updated
    rustsec_no_updates: state => {
      return state.dependencies.filter((dependency) => {
        return (
          (dependency.vulnerabilities != null ||
            dependency.warnings != null) &&
          dependency.update == null
        );
      });
    },
    // dependencies that can be updated
    can_update_dependencies(state, getters) {
      return getters.updatable_dependencies.filter(
        (dependency) => dependency.update_allowed
      );
    },
    // non-dev dependencies that have an update
    non_dev_updatable_deps(state, getters) {
      let non_dev_updatable_deps = getters.can_update_dependencies.filter(
        (dependency) => !dependency.dev
      );
      return non_dev_updatable_deps.sort(
        sort_priority
      );
    },
    // dev dependencies that have an update
    dev_updatable_deps(state, getters) {
      let dev_updatable_deps = getters.can_update_dependencies.filter(
        (dependency) => dependency.dev
      );
      return dev_updatable_deps.sort(sort_priority);
    },
    // dependencies that have updates and _can't_ be updated
    cant_update_deps(state, getters) {
      let cant_update_deps = getters.updatable_dependencies.filter(
        (dependency) => !dependency.update_allowed
      );
      return cant_update_deps.sort(sort_priority);
    },
    // get dependency
    dependency: (state) => (dep) => {
      return state.dependency_map[dep];
    }
  },
  mutations: {
    add_analysis(state, analysis) {
      // reset state if we're retrieving a different repo
      if (state.repo != "" && analysis.repository != state.repo) {
        Object.assign(state, getDefaultState());
      }

      // extract
      console.log("analysis data: ", analysis);
      state.repo = analysis.repository;
      state.commit = analysis.commit;
      state.date = new Date(analysis.timestamp).toString();
      state.change_summary = analysis.rust_dependencies.change_summary || {new_updates:[], new_rustsec:{vulnerabilities:[], warnings:[]}};
      let dependencies = analysis.rust_dependencies.dependencies;
      state.rustsec = analysis.rust_dependencies.rustsec;

      // transform
      transform_analysis(dependencies, state.rustsec);

      // create map depkey -> dep
      let dependency_map = {};
      dependencies.forEach((dep) => {
        let key = `${dep.name}-${dep.version}-${dep.direct}-${dep.dev}`;
        dep.key = key;
        dependency_map[key] = dep;
      })
      state.dependency_map = dependency_map;

      // finally, set dependencies
      state.dependencies = dependencies;
    },
  },
  actions: {
    get_analysis({ commit }, repo) {
      return axios
        .get("/dependencies?repo=" + repo)
        .then((response) => {
          //
          // Error handling
          //

          // TODO: return an error code from the server instead?
          if (response.data.constructor == String) {
            return {
              "error": response.data,
            };
          }

          //
          // Retrieving data
          //

          commit("add_analysis", response.data);

          //
          // alert on vuln
          //

          if (
            response.data.rust_dependencies.rustsec.vulnerabilities.length > 0
          ) {
            return {
              "rustsec": this.rustsec.vulnerabilities
                .map((vuln) => vuln.advisory.id)
                .join(", "),
            };
          }

          // notification
          return {
            "success": true,
          };
        })
        .catch((error) => {
          return {
            "error": error
          };
        });
    },
  }
});
