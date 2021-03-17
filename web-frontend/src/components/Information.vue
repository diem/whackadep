<template>
  <section class="alert alert-warning">
    <h2 class="alert-heading">Information</h2>
    date: {{ $store.state.date }}
    <br />
    commit:
    <a
      :href="
        $store.state.repo.replace('.git', '') + '/commit/' + $store.state.commit
      "
      target="_blank"
      ><code>{{ $store.state.commit }}</code></a
    ><br />

    <div v-if="new_updates.length > 0">
      <hr />
      <h3>New updates</h3>
      <ul>
        <li
          v-for="d in new_updates"
          :key="d.name + d.version + d.direct + d.dev"
        >
          [{{ d.direct ? "direct" : "transitive" }}]
          <a :href="'#' + d.name + d.version + d.direct + d.dev">{{
            d.name
          }}</a>
          (<small>{{ d.version }} → {{ d.update.versions.join(" → ") }}</small
          >)
        </li>
      </ul>
    </div>

    <div v-if="new_vulnerabilities.length > 0">
      <hr />
      <h3>New vulnerabilities</h3>
      <li v-for="r in new_vulnerabilities" :key="r.advisory.id">
        <strong>{{ r.advisory.id }}</strong> - {{ r.advisory.title }}
      </li>
    </div>

    <div v-if="new_warnings.length > 0">
      <hr />
      <h3>Mew warnings</h3>
      <li v-for="r in new_warnings" :key="r.advisory.id">
        <strong>{{ r.advisory.id }}</strong> - {{ r.advisory.title }}
      </li>
    </div>
  </section>
</template>

<script>
export default {
  name: "Information",
  computed: {
    new_updates() {
      return this.$store.state.change_summary.new_updates;
    },
    new_vulnerabilities() {
      return this.$store.state.change_summary.new_rustsec.vulnerabilities;
    },
    new_warnings() {
      return this.$store.state.change_summary.new_rustsec.warnings;
    },
  },
};
</script>