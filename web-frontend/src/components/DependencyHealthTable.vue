<template>
    <div>
        <table
      class="table table-light table-striped table-hover table-bordered table-sm align-middle"
      style="table-layout: auto"
    >
      <thead style="position: sticky; top: 0" class="thead-dark">
        <tr>
          <th class="header text-center" scope="col">name</th>
          <th class="header text-center" scope="col">version</th>
          <th class="header text-center" scope="col">type</th>
          <th class="header text-center" scope="col">unsafe lines</th>
          <th class="header text-center" scope="col">forbids unsafe?</th>
          <th class="header text-center" scope="col">lines of code</th>
          <th class="header text-center" scope="col">downloads</th>
          <th class="header text-center" scope="col">github stars</th>
          <th class="header text-center" scope="col">dependant crates</th>
          <th class="header text-center" scope="col">activity level</th>
          <th class="header text-center" scope="col">OSSF</th>
          <th class="header text-center" scope="col">build.rs</th>
          <th class="header text-center" scope="col">git tag</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="d in dependencies"
          :key="d.name + d.version + d.direct + d.dev"
        >
          <td> {{ d["name"] }}</td>
          <td> {{ d["version"] }}</td>
          <td> {{ d["direct"] ? "direct" : "transitive" }} </td>
        </tr>
      </tbody>
      </table>
    </div>
</template>

<script>
export default {
    name: "DependencyHealthTable",
    props: {
        repo: String,
        dependencies: Array,
    },
};
</script>
