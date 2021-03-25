<template>
  <div v-if="dependency && dependency.update">
    <h2>Review</h2>

    <!-- risk -->
    <section v-if="dependency.risk_score > 0">
      <h3>Risk</h3>
      <ul>
        <li v-for="reason in dependency.risk_reasons" :key="reason">
          <p v-html=reason></p>
        </li>
      </ul>
    </section>

    <!-- git stuff -->
    <section v-if="changelog_text">
      <h3>Changelog</h3>

      <p>
        Note that you should not take that information at face value, as it
        could have been written with the intention to deceive.
      </p>

      {{ changelog_text }}

      <h3>Commits</h3>

      <ul>
        <li v-for="commit in commits" :key="commit.html_url">
          <a :href="commit.html_url" target="_blank">{{ commit.message }}</a>
        </li>
      </ul>

      <p>
        Note that these <strong>Github</strong> commits might have no relation
        what-so-ever with the actual code pulled from <strong>crates.io</strong>
      </p>
    </section>

    <!-- create PR -->
    <section>
      <h3>Create a PR</h3>
      <p>
        To create a PR easily, make sure you are on an up-to-date branch of the
        <code>main</code> branch and paste the following in your terminal:
      </p>

      <pre><code>cargo update-dep -p {{dependency.name}} -v {{dependency.version}} -n {{new_version}}</code></pre>

      <p>
        This assumes that you have cargo-update-dep installed, you can get it
        via:
      </p>

      <pre><code>cargo install cargo-update-dep</code></pre>
    </section>
  </div>
</template>

<script>
export default {
  name: "Review",
  props: {
    depkey: String,
  },
  computed: {
    dependency() {
      return this.$store.getters.dependency(this.depkey);
    },
    new_version() {
      if (
        this.dependency.update &&
        this.dependency.update.versions &&
        this.dependency.update.versions.length > 0
      ) {
        return this.dependency.update.versions[
          this.dependency.update.versions.length - 1
        ];
      } else {
        return null;
      }
    },
    changelog_text() {
      if (
        this.dependency.update &&
        this.dependency.update.update_metadata &&
        this.dependency.update.update_metadata.changelog_text
      ) {
        return this.dependency.update.update_metadata.changelog_text;
      }
      return null;
    },
    commits() {
      if (
        this.dependency.update &&
        this.dependency.update.update_metadata &&
        this.dependency.update.update_metadata.commits
      ) {
        return this.dependency.update.update_metadata.commits;
      }
      return null;
    },
  },
};
</script>
