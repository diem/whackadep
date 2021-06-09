<template>
  <section>
  <Information />

    <hr />

    <!-- statistics -->
    <Statistics :dependencies="$store.state.dependencies" />

    <hr />
    <DependencyHealthTable
      :repo="repo"
      :dependencies="$store.state.dependencies"
    />
  </section>

</template>

<script>
import DependencyHealthTable from './DependencyHealthTable.vue';
import Information from "./Information.vue"
import Statistics from "./Statistics.vue"

export default {
  name: "DependencyHealthDashboard",
  props: {
    repo: String,
  },
  components: {
    Information,
    Statistics,
    DependencyHealthTable,
  },
};

</script>
