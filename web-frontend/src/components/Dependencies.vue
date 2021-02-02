<template>
  <div>
    <table
      class="table table-light table-striped table-hover table-bordered table-sm align-middle"
      style="table-layout: fixed"
    >
      <thead style="position: sticky; top: 0">
        <tr>
          <th class="header" scope="col">name</th>
          <th class="header" scope="col">type</th>
          <th class="header" scope="col">version change</th>
          <th class="header" scope="col">rustsec</th>
          <th class="header" scope="col">update</th>
          <th class="header" scope="col">changelog</th>
          <th class="header" scope="col">commits</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="d in dependencies" v-bind:key="d.name">
          <td>{{ d.name }}</td>
          <td>
            <span v-if="d.direct">direct</span><span v-else>transitive</span>
          </td>
          <td>
            <span
              v-if="d.update"
              :title="d.version + ' → ' + d.update.versions.join(' → ')"
            >
              {{ version_change(d) }}
            </span>
          </td>
          <td>
            <span v-if="d.rustsec" :title="JSON.stringify(d.rustsec)">
              <strong>{{ d.rustsec.advisory.id }}</strong
              >: {{ d.rustsec.advisory.title }}.

              <span v-if="d.rustsec.version_info.patched.length > 0">
                <br />versions patched:
                {{ d.rustsec.version_info.patched.join(", ") }}.
              </span>

              <span v-if="d.rustsec.version_info.unaffected.length > 0">
                <br />versions unaffected:
                {{ d.rustsec.version_info.unaffected.join(", ") }}
              </span>
            </span>
          </td>
          <td>
            <a
              v-if="d.update"
              @click.prevent="
                $refs.modal.open(
                  d.name,
                  d.version,
                  d.update.versions[d.update.versions.length - 1]
                )
              "
              href="#"
              >create a PR</a
            >
            <span class="invisible">{{ d.create_PR }}</span>
          </td>
          <td>
            <span
              v-if="d.update && d.update.update_metadata.changelog_text"
              :title="d.update.update_metadata.changelog_text"
            >
              {{ clean_changelog(d.update.update_metadata.changelog_text) }}
              <a :href="d.update.update_metadata.changelog_url">[...]</a>
            </span>
          </td>
          <td>
            <span
              v-if="
                d.update &&
                d.update.update_metadata.commits &&
                d.update.update_metadata.commits.length > 0
              "
              :title="JSON.stringify(d.update.update_metadata.commits)"
            >
              <a :href="d.update.update_metadata.commits_url">commits</a>
            </span>
          </td>
        </tr>
      </tbody>
    </table>
    <Modal ref="modal" />
  </div>
</template>

<script>
import Modal from "./Modal.vue";

export default {
  name: "DependenciesTable",
  props: {
    dependencies: [],
  },
  components: {
    Modal,
  },
  inject: {
    semver: {
      from: "semver",
    },
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

      var type_change = this.semver.diff(version, new_version);
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