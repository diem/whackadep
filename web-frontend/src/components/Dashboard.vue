<template>
  <section>
    <p>Use this dashboard to manage your dependencies.</p>
    <p>commit: {{ commit }}</p>
    <p># non-dev direct dependencies: {{ direct_dependencies }}</p>
    <p># non-dev transitive dependencies: {{ transitive_dependencies }}</p>
    <p># direct dev dependencies: {{ dev_dependencies }}</p>
    <h2>updates available</h2>
    <table class="table table-light table-striped table-hover">
      <thead style="position: sticky; top: 0">
        <tr>
          <th class="header" scope="col">name</th>
          <th class="header" scope="col">direct?</th>
          <th class="header" scope="col">dev?</th>
          <th class="header" scope="col">version change</th>
          <th class="header" scope="col">rustsec</th>
          <th class="header" scope="col">create PR (unless review needed)</th>
          <th class="header" scope="col">changelog</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="d in updatable_dependencies" v-bind:key="d.name">
          <td>{{ d.name }}</td>
          <td>{{ d.direct }}</td>
          <td>{{ d.dev }}</td>
          <td>{{ d.version }} -> {{ d.new_version }}</td>
          <td>{{ d.rustsec }}</td>
          <td>
            <a @click="copy_to_clipboard">click to create a PR</a>
            <span class="invisible">{{ d.create_PR }}</span>
          </td>
          <td></td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<script>
import axios from "axios";

export default {
  name: "Dashboard",
  data() {
    return {
      commit: null,
      dependencies: null,
      updatable_dependencies: null,
    };
  },
  mounted() {
    axios.get("/dependencies").then((response) => {
      this.commit = response.data.commit;
      this.dependencies = response.data.rust_dependencies.dependencies;
      this.updatable_dependencies = response.data.rust_dependencies.dependencies.filter(
        (dependency) => dependency.new_version != null
      );
    });
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