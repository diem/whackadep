<template>
  <section>
    <p>Use this dashboard to manage your dependencies.</p>
    <p>commit: {{ commit }}</p>
    <p># non-dev direct dependencies: {{ direct_dependencies }}</p>
    <p># non-dev transitive dependencies: {{ transitive_dependencies }}</p>
    <p># direct dev dependencies: {{ dev_dependencies }}</p>
    <ul>
      <li>backlog over time (graph)</li>
      <li>number of direct deps over time (graph)</li>
      <li>number of total deps over time (graph)</li>
    </ul>
    <h2>updates available</h2>
    <table class="table">
      <thead>
        <tr>
          <td>name</td>
          <td>current version</td>
          <td>new version</td>
          <td>type of change (MAJOR/MINOR/PATCH)</td>
          <td>breaking change?</td>
          <td>versions behind?</td>
          <td>RUST-SEC?</td>
          <td>create PR (unless review needed)</td>
          <td>changelog</td>
        </tr>
      </thead>
      <tbody>
        <tr v-for="d in dependencies" v-bind:key="d.name">
          <td>{{ d.name }}</td>
          <td>{{ d.version }}</td>
          <td>{{ d.new_version }}</td>
          <td></td>
          <td></td>
          <td></td>
          <td></td>
          <td></td>
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
    };
  },
  mounted() {
    axios.get("/dependencies").then((response) => {
      this.commit = response.data.commit;
      this.dependencies = response.data.rust_dependencies.dependencies;
    });
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
