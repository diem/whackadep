<template>
  <div>
    <table
      class="table table-light table-striped table-hover table-bordered table-sm align-middle"
    >
      <thead style="position: sticky; top: 0" class="thead-dark">
        <tr>
          <th class="header" scope="col">name</th>
          <th class="header" scope="col">type</th>
          <th class="header" scope="col">rustsec</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="d in dependencies" v-bind:key="d.name">
          <td>
            <strong>{{ d.name }}</strong>
          </td>
          <td>
            {{ d.direct ? "direct" : "transitive" }}
          </td>
          <td>
            <span v-if="d.rustsec">
              <div v-for="rustsec in d.rustsec" :key="rustsec.advisory.id">
                <strong
                  ><a
                    :href="
                      'https://rustsec.org/advisories/' +
                      rustsec.advisory.id +
                      '.html'
                    "
                    target="_blank"
                    >{{ rustsec.advisory.id }}</a
                  ></strong
                >: {{ rustsec.advisory.title }}.

                <span v-if="rustsec.versions.patched.length > 0">
                  <br />versions patched:
                  {{ rustsec.versions.patched.join(", ") }}.
                </span>

                <span v-if="rustsec.versions.unaffected.length > 0">
                  <br />versions unaffected:
                  {{ rustsec.versions.unaffected.join(", ") }}
                </span>
              </div>
            </span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script>
import semver from "semver";

export default {
  name: "DependenciesTable",
  props: {
    dependencies: Array,
  },
  methods: {
    clean_changelog(changelog) {
      var res = changelog.replaceAll(/(#)*/g, "");
      res = res.replaceAll(/\[([^\]]+)\]\([^)]+\)/g, "$1");
      return res.slice(0, 100);
    },
    version_change(dependency) {
      var version = dependency.version;
      var new_version =
        dependency.update.versions[dependency.update.versions.length - 1];
      // rust has the tendency to lie when

      var type_change = semver.diff(version, new_version);
      return type_change + " (" + version + " → " + new_version + ")";
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