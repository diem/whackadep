<template>
  <table class="table table-light table-striped table-hover">
    <thead style="position: sticky; top: 0">
      <tr>
        <th class="header" scope="col">name</th>
        <th class="header" scope="col">direct?</th>
        <th class="header" scope="col">dev?</th>
        <th class="header" scope="col">version change</th>
        <th class="header" scope="col">rustsec</th>
        <th class="header" scope="col">update</th>
        <th class="header" scope="col">changelog</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="d in dependencies" v-bind:key="d.name">
        <td>{{ d.name }}</td>
        <td>{{ d.direct }}</td>
        <td>{{ d.dev }}</td>
        <td>{{ d.version }} -> {{ d.new_version.versions.join(", ") }}</td>
        <td>{{ d.rustsec }}</td>
        <td>
          <a @click="copy_to_clipboard">click to create a PR</a>
          <span class="invisible">{{ d.create_PR }}</span>
        </td>
        <td></td>
      </tr>
    </tbody>
  </table>
</template>

<script>
export default {
  name: "DependenciesTable",
  props: {
    dependencies: [],
  },
  setup(props) {
    console.log(props.dependencies);
  },
};
</script>

<style scoped>
.header {
  position: sticky;
  top: 0;
}
</style>