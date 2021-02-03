<template>
  <div>
    <table
      class="table table-light table-striped table-hover table-bordered table-sm align-middle"
      style="table-layout: fixed"
    >
      <thead style="position: sticky; top: 0">
        <tr>
          <th class="header text-center" scope="col">priority</th>
          <th class="header text-center" scope="col">name</th>
          <th class="header text-center" scope="col">type</th>
          <th class="header text-center" scope="col">version change</th>
          <th class="header text-center" scope="col">rustsec</th>
          <th class="header text-center" scope="col">update</th>
          <th class="header text-center" scope="col">changelog</th>
          <th class="header text-center" scope="col">commits</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(d, index) in dependencies"
          v-bind:key="d.name + d.version + d.direct + d.dev"
        >
          <!-- rank -->
          <th scope="row" class="text-center">
            <a
              href="#"
              @click.prevent
              v-b-popover.hover.top="d.priority_reasons.join('\n')"
              >{{ index + 1 }}</a
            >
          </th>
          <!-- name -->
          <td>{{ d.name }}</td>
          <td>
            <span v-if="d.direct">direct</span><span v-else>transitive</span>
          </td>
          <!-- version -->
          <td>
            <span
              v-b-tooltip.hover="
                d.version + ' → ' + d.update.versions.join(' → ')
              "
              v-text="version_change(d)"
            ></span>
          </td>
          <!-- RUSTSEC -->
          <td>
            <span
              v-if="d.rustsec"
              v-b-popover.hover.top="clean_rustsec(d.rustsec)"
            >
              <strong>{{ d.rustsec.advisory.id }}</strong>
            </span>
          </td>
          <!-- create PR -->
          <td class="text-center">
            <a v-if="d.update" href="#">create a PR</a>
            <span class="invisible">{{ d.create_PR }}</span>
          </td>
          <!-- changelog -->
          <td class="text-center">
            <span
              v-if="d.update && d.update.update_metadata.changelog_text"
              :title="d.update.update_metadata.changelog_text"
            >
              <a
                href="#"
                @click.prevent
                v-b-popover.hover.top="
                  clean_changelog(d.update.update_metadata.changelog_text)
                "
                >preview</a
              >
              (<a :href="d.update.update_metadata.changelog_url">link</a>)
            </span>
          </td>
          <!-- commits -->
          <td class="text-center">
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
      // strip markdown links
      res = res.replaceAll(/\[([^\]]+)\]\([^)]+\)/g, "$1");
      // strip html tags
      res = res.replace(/(<([^>]+)>)/gi, "");
      //
      return res.slice(0, 100) + " [...]";
    },
    version_change(dependency) {
      var version = dependency.version;
      var new_version =
        dependency.update.versions[dependency.update.versions.length - 1];
      // rust has the tendency to lie when

      var type_change = semver.diff(version, new_version);
      return type_change;
    },
    clean_rustsec(rustsec) {
      let title = rustsec.advisory.title;
      let desc = rustsec.advisory.description;

      let result = `${title}, ${desc}`;

      if (rustsec.version_info.patched.length > 0) {
        let patched = rustsec.version_info.patched.join(", ");
        result += `, versions patched: ${patched}.`;
      }

      if (rustsec.version_info.unaffected.length > 0) {
        let unaffected = rustsec.version_info.unaffected.join(", ");
        result += `, versions unaffected: ${unaffected}.`;
      }

      return result;
    },
  },
};
</script>

<style scoped>
.header {
  position: sticky;
  top: 0;
}

a {
  text-decoration: none;
}
</style>
