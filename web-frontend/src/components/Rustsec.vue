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
            <!-- vulnerabilities -->
            <div v-for="vuln in d.vulnerabilities" :key="vuln.advisory.id">
              <strong>
                <a
                  :href="
                    'https://rustsec.org/advisories/' +
                    vuln.advisory.id +
                    '.html'
                  "
                  target="_blank"
                >
                  {{ vuln.advisory.id }}
                </a>
              </strong>
              : {{ vuln.advisory.title }}.

              <div v-if="vuln.versions.patched.length > 0">
                <br />versions patched: {{ vuln.versions.patched.join(", ") }}.
              </div>

              <div v-if="vuln.versions.unaffected.length > 0">
                <br />versions unaffected:
                {{ vuln.versions.unaffected.join(", ") }}
              </div>
            </div>
            <!-- warnings -->
            <div v-for="warning in d.warnings" :key="warning.package.name">
              <span v-if="warning.advisory">
                <strong>
                  <a
                    :href="
                      'https://rustsec.org/advisories/' +
                      warning.advisory.id +
                      '.html'
                    "
                    target="_blank"
                  >
                    {{ warning.advisory.id }}
                  </a>
                </strong>
                : {{ warning.advisory.title }}.
              </span>

              <div v-if="warning.kind != 'unmaintained'">
                <div
                  v-if="warning.versions && warning.versions.patched.length > 0"
                >
                  versions patched:
                  {{ warning.versions.patched.join(", ") }}.
                </div>

                <div
                  v-if="
                    warning.versions && warning.versions.unaffected.length > 0
                  "
                >
                  versions unaffected:
                  {{ warning.versions.unaffected.join(", ") }}
                </div>
              </div>
            </div>
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
      let res = changelog.replaceAll(/(#)*/g, "");
      res = res.replaceAll(/\[([^\]]+)\]\([^)]+\)/g, "$1");
      return res.slice(0, 100);
    },
    version_change(dependency) {
      let version = dependency.version;
      let new_version =
        dependency.update.versions[dependency.update.versions.length - 1];
      // rust has the tendency to lie when

      let type_change = semver.diff(version, new_version);
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