<template>
  <div class="container">
    <!-- header/nav -->
    <nav class="navbar navbar-expand-lg navbar-dark bg-dark">
      <span class="navbar-brand mb-0 h1">Whack-a-dep!</span>
      <button
        class="navbar-toggler"
        type="button"
        data-toggle="collapse"
        data-target="#navbarText"
        aria-controls="navbarText"
        aria-expanded="false"
        aria-label="Toggle navigation"
      >
        <span class="navbar-toggler-icon"></span>
      </button>
      <div class="collapse navbar-collapse" id="navbarText">
        <ul class="navbar-nav mr-auto">
          <li class="nav-item active">
            <router-link to="/" class="nav-link">Home</router-link>
          </li>
          <li class="nav-item">
            <a class="nav-link" @click.prevent="refresh" href="#">Refresh</a>
          </li>
        </ul>
      </div>

      <!-- dropdown to choose repo -->
      <b-dropdown
        id="dropdownrepo"
        :text="current_repo"
        :key="current_repo"
        class="m-md-2"
      >
        <b-dropdown-item
          v-for="repo in repos"
          v-bind:key="repo"
          :disabled="repo == current_repo"
          :to="{ name: 'repo', params: { repo: repo } }"
        >
          {{ repo }}
        </b-dropdown-item>
        <b-dropdown-divider></b-dropdown-divider>
        <b-dropdown-item @click="showModal"
          >add a new rust repository</b-dropdown-item
        >
      </b-dropdown>
    </nav>

    <!-- <Dashboard :repo="current_repo" /> -->
    <router-view :key="$route.path" />

    <!-- modal -->
    <b-modal ref="modal" hide-footer title="Adding a new repo">
      <div>
        <b-form @submit="onSubmit" @reset="onReset">
          <b-form-group
            id="repo-group"
            label="Git repository:"
            label-for="input-1"
            description="We only support Rust repositories at the moment."
          >
            <b-form-input
              id="repo"
              v-model="form.repo"
              placeholder="Enter git repository"
              required
            ></b-form-input>
          </b-form-group>
          <b-button type="submit" variant="primary">Add</b-button>
        </b-form>
      </div>
    </b-modal>
  </div>
</template>

<script>
import axios from "axios";

export default {
  name: "App",
  data() {
    return {
      // repo mgmt
      current_repo: "select a repository",
      repos: [],
      form: {
        repo: "",
      },
    };
  },
  methods: {
    // obtain the repositories installed (from configuration)
    get_repos() {
      axios
        .get("/repos")
        .then((response) => {
          this.repos = response.data;
        })
        .catch((error) => {
          console.log(error);
          if (error.response) {
            // The request was made and the server responded with a status code
            // that falls out of the range of 2xx
            this.toast("Error from the server", error.message, "danger");
          } else if (error.request) {
            // The request was made but no response was received
            // `error.request` is an instance of XMLHttpRequest in the browser and an instance of
            // http.ClientRequest in node.js
            this.toast(
              "server unavailable",
              `more information: ${JSON.stringify(error.message)}`,
              "danger"
            );
          } else {
            // Something happened in setting up the request that triggered an Error
            this.toast(
              "unknown error",
              `more information: ${JSON.stringify(error.message)}`,
              "danger"
            );
          }
          console.log(error.config);
        });
    },

    // attempts to start an analysis on a given repo
    refresh() {
      axios.get("/refresh?repo=" + this.current_repo).then((response) => {
        if (response.data == "ok") {
          this.toast(
            "Refresh requested",
            "analysis started, please refresh the page in a bit...",
            "success"
          );
        } else {
          this.toast("Refresh requested", response.data, "info");
        }
      });
    },

    // modal
    onSubmit(event) {
      event.preventDefault();
      this.hideModal();
      axios
        .post("/add_repo", this.form)
        .then((response) => {
          // TODO: return an error code from the server instead?
          if (response.data == "ok") {
            this.toast("Git repository added", "success", "success");
            this.get_repos();
          } else {
            this.toast(
              "Problem adding git repository",
              response.data,
              "danger"
            );
          }
        })
        .catch((error) => {
          console.log(error);
          if (error.response) {
            // The request was made and the server responded with a status code
            // that falls out of the range of 2xx
            this.toast("Error from the server", error.message, "danger");
          } else if (error.request) {
            // The request was made but no response was received
            // `error.request` is an instance of XMLHttpRequest in the browser and an instance of
            // http.ClientRequest in node.js
            this.toast(
              "server unavailable",
              `more information: ${JSON.stringify(error.message)}`,
              "danger"
            );
          } else {
            // Something happened in setting up the request that triggered an Error
            this.toast(
              "unknown error",
              `more information: ${JSON.stringify(error.message)}`,
              "danger"
            );
          }
          console.log(error.config);
        });
      this.onReset();
    },
    onReset() {
      // Reset our form values
      this.form.repo = "";
    },
    create_repo() {
      this.showModal();
    },
    showModal() {
      this.$refs["modal"].show();
    },
    hideModal() {
      this.$refs["modal"].hide();
      this.modal_text = "";
    },

    // helpers

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

  mounted() {
    this.get_repos();

    // current repo
    if (this.$route.name == "repo") {
      this.current_repo = this.$route.params.repo;
    }
  },

  watch: {
    $route(to, from) {
      console.log("route change from", from, " to", to);

      // new repo
      if (to.name == "repo") {
        this.current_repo = to.params.repo;
      }

      //
      if (to.name == "landing") {
        this.current_repo = "select a repository";
      }
    },
  },
};
</script>
