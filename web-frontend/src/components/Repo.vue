<template>
  <router-view></router-view>
</template>

<script>
export default {
  name: "Repo",
  props: {
    repo: String,
  },
  created() {
    if (this.repo == undefined) {
      this.toast("error", "error", "danger");
    }

    // get dependencies for a repo
    this.$store.dispatch("get_analysis", this.repo).then((res) => {
      // error
      if ("error" in res) {
        this.toast("error", res["error"], "danger");
      } else {
        if ("rustsec" in res) {
          // rustsec detected
          this.toast(
            "RUSTSEC",
            `vulnerabilities found: ${res["rustsec"]}`,
            "danger"
          );
        }

        // success
        this.toast(
          "Retrieving analysis",
          `latest analysis successfuly retrieved for ${this.repo}`,
          "success"
        );
      }
    });
  },
  methods: {
    // create a toast (a notification on the top right of the screen)
    toast(title, msg, variant = null) {
      this.$bvToast.toast(msg, {
        title: title,
        autoHideDelay: 5000,
        appendToast: true,
        variant: variant,
        solid: true,
      });
    },
  },
};
</script>