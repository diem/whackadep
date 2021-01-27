<template>
  <section>
    <p>
      commit: <code>{{ commit }}</code>
    </p>

    <div class="row" id="stats">
      <div class="col-sm bg-warning bg-gradient p-5 center">
        <big>{{ direct_dependencies }} </big>
        <small> non-dev direct dependencies</small>
      </div>
      <div class="col-sm p-5 bg-info bg-gradient">
        <big>{{ transitive_dependencies }} </big>
        <small> non-dev transitive dependencies</small>
      </div>
      <div class="col-sm bg-success bg-gradient p-5">
        <big>{{ dev_dependencies }} </big>
        <small> direct dev dependencies</small>
      </div>
    </div>

    <h2>RUSTSEC advisories without updates</h2>
    <DependenciesTable v-bind:dependencies="rustsec" />

    <h2>
      Updates available for non-dev dependencies ({{
        non_dev_updatable_deps_count
      }})
    </h2>
    <DependenciesTable v-bind:dependencies="non_dev_updatable_deps" />

    <h2>
      Updates available for dev dependencies ({{ dev_updatable_deps_count }})
    </h2>
    <DependenciesTable v-bind:dependencies="dev_updatable_deps" />
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
  },
  mounted() {
    this.axios.get("/dependencies").then((response) => {
      this.commit = response.data.commit;
      this.dependencies = response.data.rust_dependencies.dependencies;
      var updatable_dependencies = response.data.rust_dependencies.dependencies.filter(
        (dependency) => dependency.new_version != null
      );
      this.non_dev_updatable_deps = updatable_dependencies.filter(
        (dependency) => !dependency.dev
      );
      this.dev_updatable_deps = updatable_dependencies.filter(
        (dependency) => dependency.dev
      );
      this.rustsec = response.data.rust_dependencies.dependencies.filter(
        (dependency) =>
          dependency.rustsec != null && dependency.new_version == null
      );
    });
  },
  components: {
    DependenciesTable,
  },
  methods: {
    copy_to_clipboard() {
      /*
        try {
          // Now that we've selected the anchor text, execute the copy command
          var successful = document.execCommand("copy");
          var msg = successful ? "successful" : "unsuccessful";
          console.log("Copy email command was " + msg);
        } catch (err) {
          console.log("Oops, unable to copy");
        }

        // Remove the selections - NOTE: Should use
        // removeRange(range) when it is supported
        window.getSelection().removeAllRanges();
      });
      */
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
    non_dev_updatable_deps_count() {
      if (this.non_dev_updatable_deps != null) {
        return this.non_dev_updatable_deps.length;
      } else {
        return 0;
      }
    },
    dev_updatable_deps_count() {
      if (this.dev_updatable_deps != null) {
        return this.dev_updatable_deps.length;
      } else {
        return 0;
      }
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