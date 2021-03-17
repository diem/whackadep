<template>
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
</template>

<script>
export default {
  name: "Statistics",

  props: {
    dependencies: Array,
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
#stats {
  text-align: center;
  margin-bottom: 20px;
}
#stats div {
  margin: 0 5px;
}
</style>
      