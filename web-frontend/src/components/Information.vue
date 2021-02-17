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

    <div v-if="length($store.state.change_summary.new_updates) > 0">
      <hr />
      new updates:
      <ul>
        <li
          v-for="d in $store.state.change_summary.new_updates"
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

    <div v-if="length($store.state.change_summary.new_rustsec) > 0">
      <hr />
      new RUSTSEC advisories:
      <li
        v-for="r in $store.state.change_summary.new_rustsec"
        :key="r.advisory.id"
      >
        <strong>{{ r.advisory.id }}</strong> - {{ r.advisory.title }}
      </li>
    </div>
  </section>
</template>

<script>
export default {
  name: "Information",
  methods: {
    // length or 0 if undefined
    length(thing) {
      if (typeof thing == Array) {
        return thing.length;
      } else {
        return 0;
      }
    },
  },
};
</script>