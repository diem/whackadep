<template>
  <div>
    <table
      class="table table-light table-striped table-hover table-bordered table-sm align-middle"
      style="table-layout: fixed"
    >
      <thead style="position: sticky; top: 0" class="thead-dark">
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
          :key="d.name + d.version + d.direct + d.dev"
        >
          <!-- rank -->
          <th scope="row" class="text-center">
            <a
              href="#"
              @click.prevent
              v-b-popover.hover.left="d.priority_reasons.join(', ')"
              >{{ index + 1 }}</a
            >
          </th>
          <!-- name -->
          <td>
            <strong :id="d.name + d.version + d.direct + d.dev">{{
              d.name
            }}</strong>
          </td>
          <td>
            {{ d.direct ? "direct" : "transitive" }}
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
            <div v-if="d.vulnerabilities">
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
              </div>
            </div>

            <div v-if="d.warnings">
              <div v-for="warning in d.warnings" :key="warning.package.name">
                <strong>
                  <a
                    v-if="warning.advisory"
                    :href="
                      'https://rustsec.org/advisories/' +
                      warning.advisory.id +
                      '.html'
                    "
                    target="_blank"
                  >
                    {{ warning.advisory.id }}
                  </a>
                  {{ warning.kind }}
                </strong>
              </div>
            </div>
          </td>
          <!-- create PR -->
          <td class="text-center">
            <router-link
              v-if="d.update_allowed"
              :to="{
                name: 'review',
                params: {
                  depkey: `${d.name}-${d.version}-${d.direct}-${d.dev}`,
                },
              }"
            >
              <span v-if="d.risk_score > 0">review</span
              ><span v-else>create PR</span>
            </router-link>
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
              >
                preview
              </a>
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
    repo: String,
    dependencies: Array,
  },
  data() {
    return {
      modal_text: "",
    };
  },
  methods: {
    //
    clean_changelog(changelog) {
      let res = changelog.replaceAll(/(#)*/g, "");
      // strip markdown links
      res = res.replaceAll(/\[([^\]]+)\]\([^)]+\)/g, "$1");
      // strip html tags
      res = res.replace(/(<([^>]+)>)/gi, "");
      //
      return res.slice(0, 100) + " [...]";
    },
    version_change(dependency) {
      let version = dependency.version;
      let new_version =
        dependency.update.versions[dependency.update.versions.length - 1];
      // rust has the tendency to lie when

      let type_change = semver.diff(version, new_version);
      return type_change;
    },
    clean_rustsec(rustsec) {
      let result = "yanked";
      if (rustsec.advisory) {
        let title = rustsec.advisory.title;
        //      let desc = rustsec.advisory.description;

        result = `${title}`;
      }

      if (rustsec.versions) {
        if (rustsec.versions.patched.length > 0) {
          let patched = rustsec.versions.patched.join(", ");
          result += `, versions patched: ${patched}.`;
        }

        if (rustsec.versions.unaffected.length > 0) {
          let unaffected = rustsec.versions.unaffected.join(", ");
          result += `, versions unaffected: ${unaffected}.`;
        }
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
