<template>
  <div class="hello">
    <p>Use this dashboard to manage your dependencies.</p>
    <p>commit: {{ commit }}</p>
    <ul>
      <li>backlog over time (graph)</li>
      <li>number of direct deps over time (graph)</li>
      <li>number of total deps over time (graph)</li>
    </ul>
    <h2>updates available</h2>
    <table>
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
    </table>
  </div>
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
};
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped>
h3 {
  margin: 40px 0 0;
}
ul {
  list-style-type: none;
  padding: 0;
}
li {
  display: inline-block;
  margin: 0 10px;
}
a {
  color: #42b983;
}
</style>
