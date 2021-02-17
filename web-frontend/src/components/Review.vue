<template>
  <div>
    <h2>Review</h2>

    <h3>Risk</h3>

    <h3>Changelog</h3>

    <p>
      Note that you should not take that information at face value, as it could
      have been written with the intention to deceive.
    </p>

    <h3>Commits</h3>

    <p>
      Note that these <strong>Github</strong> commits might have no relation
      what-so-ever with the actual code pulled from <strong>crates.io</strong>
    </p>

    <h3>Create a PR</h3>
    <p>
      To create a PR easily, make sure you are on an up-to-date branch of the
      <code>main</code> branch and paste the following in your terminal:
    </p>

    <pre><code>cargo update-dep -p ${{dependency.name}} -v ${{dependency.version}} -n ${{dependency.new_version}}</code></pre>

    <p>
      This assumes that you have cargo-update-dep installed, you can get it via:
    </p>

    <pre><code>cargo install cargo-update-dep</code></pre>
  </div>
</template>

<script>
export default {
  name: "Review",
  data() {
    return {
      dependency: {},
    };
  },
  props: {
    repo: String,
    depkey: String,
  },
  mounted() {
    if (this.repo != this.$store.state.repo) {
      alert("error, refresh repo analysis first");
    }
    console.log("looking for dep", this.depkey);
    let dependency = this.$store.state.dependency_map[this.depkey];
    console.log("found dependency:", dependency);
    this.dependency = dependency;
  },
};
</script>