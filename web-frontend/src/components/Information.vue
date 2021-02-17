<template>
  <section class="alert alert-warning">
    <h2 class="alert-heading">Information</h2>
    date: {{ date }}
    <br />
    commit:
    <a :href="repo.replace('.git', '') + '/commit/' + commit" target="_blank"
      ><code>{{ commit }}</code></a
    ><br />
    <div v-if="change_summary">
      <div v-if="change_summary.new_updates.length > 0">
        <hr />
        new updates:
        <ul>
          <li
            v-for="d in change_summary.new_updates"
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

      <div v-if="change_summary.new_rustsec.length > 0">
        <hr />
        new RUSTSEC advisories:
        <li v-for="r in change_summary.new_rustsec" :key="r.advisory.id">
          <strong>{{ r.advisory.id }}</strong> - {{ r.advisory.title }}
        </li>
      </div>
    </div>
  </section>
</template>

<script>
export default {
  name: "Information",
  props: {
    date: String,
    repo: String,
    commit: String,
    change_summary: Object,
  },
};
</script>