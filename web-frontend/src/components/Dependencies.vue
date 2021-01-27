<template>
  <div>
    <table
      class="table table-light table-striped table-hover table-bordered table-sm align-middle"
    >
      <thead style="position: sticky; top: 0">
        <tr>
          <th class="header" scope="col">name</th>
          <th class="header" scope="col">type</th>
          <th class="header" scope="col">version change</th>
          <th class="header" scope="col">rustsec</th>
          <th class="header" scope="col">update</th>
          <th class="header" scope="col">changelog</th>
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
              v-if="d.new_version"
              :title="d.version + ' → ' + d.new_version.versions.join(' → ')"
            >
              <span>
                {{ d.version }} →
                {{ d.new_version.versions[d.new_version.versions.length - 1] }}
              </span>
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
              v-if="d.new_version"
              @click.prevent="
                $refs.modal.open(
                  d.name,
                  d.version,
                  d.new_version.versions[d.new_version.versions.length - 1]
                )
              "
              href="#"
              >create a PR</a
            >
            <span class="invisible">{{ d.create_PR }}</span>
          </td>
          <td></td>
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
};
</script>

<style scoped>
.header {
  position: sticky;
  top: 0;
}
</style>