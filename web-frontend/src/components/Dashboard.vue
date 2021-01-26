<template>
  <section>
    <p>Use this dashboard to manage your dependencies.</p>
    <p>commit: {{ commit }}</p>
    <p># non-dev direct dependencies: {{ direct_dependencies }}</p>
    <p># non-dev transitive dependencies: {{ transitive_dependencies }}</p>
    <p># direct dev dependencies: {{ dev_dependencies }}</p>
    <h2>Updates available for non-dev dependencies</h2>
    <DependenciesTable v-bind:dependencies="non_dev_updatable_deps" />
    <h2>Updates available for dev dependencies</h2>
    <DependenciesTable v-bind:dependencies="dev_updatable_deps" />
  </section>
</template>

<script>
import axios from "axios";

import DependenciesTable from "./Dependencies.vue";

export default {
  name: "Dashboard",
  data() {
    return {
      commit: null,
      dependencies: null,
      dev_updatable_deps: null,
      non_dev_updatable_deps: null,
    };
  },
  mounted() {
    axios.get("/dependencies").then((response) => {
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
        console.log(this.dependencies);
        return this.dependencies.filter((dep) => !dep.dev && dep.direct).length;
      }
      return 0;
    },
    transitive_dependencies() {
      if (this.dependencies != null) {
        // there will be redundant dependencies
        console.log(this.dependencies);
        return this.dependencies.filter((dep) => !dep.dev && !dep.direct)
          .length;
      }
      return 0;
    },
    dev_dependencies() {
      if (this.dependencies != null) {
        // there will be redundant dependencies
        console.log(this.dependencies);
        return this.dependencies.filter((dep) => dep.dev && dep.direct).length;
      }
      return 0;
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