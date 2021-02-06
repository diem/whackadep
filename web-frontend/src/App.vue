<template>
  <div class="container">
    <!-- header/nav -->
    <nav class="navbar navbar-expand-lg navbar-light bg-light">
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
            <a class="nav-link" href="/"
              >Home <span class="sr-only">(current)</span></a
            >
          </li>
          <li class="nav-item">
            <a class="nav-link" @click.prevent="refresh" href="#">Refresh</a>
          </li>
        </ul>
        <span class="navbar-text"
          ><a href="https://github.com/diem/diem" target="_blank"
            >diem/diem</a
          ></span
        >
      </div>
    </nav>

    <Dashboard />
  </div>
</template>

<script>
import Dashboard from "./components/Dashboard.vue";
import axios from "axios";

export default {
  name: "App",
  components: {
    Dashboard,
  },
  methods: {
    refresh() {
      axios.get("/refresh").then((response) => {
        this.$bvToast.toast(`response from the server: ${response.data}`, {
          title: "Refresh requested",
          autoHideDelay: 5000,
          appendToast: true,
        });
      });
    },
  },
};
</script>

<style scoped>
nav {
  margin-bottom: 10px;
}
</style>

