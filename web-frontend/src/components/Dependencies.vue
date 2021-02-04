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
          v-bind:key="d.name + d.version + d.direct + d.dev"
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
            <a v-if="d.update" href="#" @click.prevent="create_PR(d)"
              >create a PR</a
            >
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
    <b-modal ref="modal" hide-footer title="Updating a dependency">
      <div class="d-block text-center" v-html="modal_text"></div>
    </b-modal>
  </div>
</template>

<script>
import semver from "semver";

export default {
  name: "DependenciesTable",
  props: {
    dependencies: Array,
  },
  data() {
    return {
      modal_text: "",
    };
  },
  methods: {
    create_PR(d) {
      let new_version = d.update.versions[d.update.versions.length - 1];
      let modal_text = "";
      if (d.update.update_metadata) {
        if (d.update.update_metadata.changelog_text) {
          modal_text += `
          <h3>Changelog</h3>
          <pre>${d.update.update_metadata.changelog_text}</pre>
        `;
        }
        if (d.update.update_metadata.commits.length > 0) {
          modal_text += `<h3>Commits</h3><ul>`;
          d.update.update_metadata.commits.forEach((commit) => {
            modal_text += `<li>${commit}</li>`;
          });
          modal_text += `</ul>`;
        }
      }

      modal_text += `
      <h3>Create a PR</h3>
      <p>To create a PR easily, make sure you are on an up-to-date branch
      of the
      <code>main</code> branch and paste the following in your terminal:</p>

      <pre><code>cargo update-dep -p ${d.name} -v ${d.version} -n ${new_version}</code></pre>

      <p>This assumes that you have cargo-update-dep installed, you can get it via:</p>

      <pre><code>cargo install cargo-update-dep</code></pre>
`;

      this.modal_text = modal_text;
      this.showModal();
    },
    //
    // Modal stuff
    //
    showModal() {
      this.$refs["modal"].show();
    },
    hideModal() {
      this.$refs["modal"].hide();
      this.modal_text = "";
    },
    //
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
